################################################################################
################################################################################
# Imports
################################################################################

from __future__ import annotations

import abc
import datetime
import json
import logging
import os
import pathlib
import sys
import traceback
import typing

import numpy
import pydantic_xml
import tap
import torch
import torch_geometric
import torchmetrics
from sklearn.svm import SVC


################################################################################
################################################################################
# Define Parameters
################################################################################


class Config(tap.Tap):
    output_directory: pathlib.Path
    model_config: pathlib.Path
    embedding_directory: pathlib.Path
    graph_directory: pathlib.Path
    structure_directory: pathlib.Path
    exclude_projects: list[str]
    model: str
    project_legacy_mapping: str = ''


################################################################################
################################################################################
# ODEM Schema
################################################################################


class ODEM(pydantic_xml.BaseXmlModel):
    version: str = pydantic_xml.attr()
    header: Header = pydantic_xml.element()
    contexts: list[Context] = pydantic_xml.element(tag='context')


class Header(pydantic_xml.BaseXmlModel):
    created_by: CreatedBy = pydantic_xml.element(tag='created-by')


class CreatedBy(pydantic_xml.BaseXmlModel):
    exporter: Exporter = pydantic_xml.element(tag='exporter')
    provider: Provider = pydantic_xml.element(tag='provider')


class Exporter(pydantic_xml.BaseXmlModel):
    version: str = pydantic_xml.attr()
    name: str


class Provider(pydantic_xml.BaseXmlModel):
    name: str


class Context(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    containers: list[Container] = pydantic_xml.element(tag='container',
                                                       default=[])


class Container(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    classification: typing.Literal[
        'jar', 'osgi-bundle'] = pydantic_xml.attr()
    namespaces: list[Namespace] = pydantic_xml.element(tag='namespace',
                                                       default=[])


class Namespace(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    types: list[Type] = pydantic_xml.element(tag='type', default=[])


class Type(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    classification: typing.Literal[
        'class', 'interface', 'enum',
        'annotation', 'unknown'] = pydantic_xml.attr()
    visibility: typing.Literal[
        'public', 'protected', 'private', 'default'] = pydantic_xml.attr()
    dependencies: Dependencies = pydantic_xml.element()


class Dependencies(pydantic_xml.BaseXmlModel):
    count: int = pydantic_xml.attr()
    depends_on: list[DependsOn] = pydantic_xml.element(tag='depends-on',
                                                       default=[])


class DependsOn(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    classification: typing.Literal[
        'extends', 'uses', 'implements'] = pydantic_xml.attr()


ODEM.model_rebuild()
Header.model_rebuild()
CreatedBy.model_rebuild()
Exporter.model_rebuild()
Provider.model_rebuild()
Context.model_rebuild()
Container.model_rebuild()
Namespace.model_rebuild()
Type.model_rebuild()
Dependencies.model_rebuild()
DependsOn.model_rebuild()



################################################################################
################################################################################
# Data Preparation
################################################################################


def get_version_triples(path: pathlib.Path):
    versions = []
    for directory in os.listdir(path):
        cleaned = directory.removesuffix('.json').removesuffix('.odem')
        version = tuple(
            int(x) if x.isdigit() else x
            for x in cleaned.split('.')
        )
        versions.append(version)
    versions.sort()
    result = []
    for i in range(len(versions) - 3 + 1):
        triple = versions[i:i + 3]
        result.append(triple)
    return result


def find_files_for_version(project: str, version: str, *,
                           graph_directory: pathlib.Path,
                           structure_directory: pathlib.Path,
                           embeddings_directory: pathlib.Path,
                           legacy_mapping: dict[str, str]):
    structure_file = structure_directory / project / version / 'hierarchy.json'
    embedding_directory = embeddings_directory / project / version
    inner_project = legacy_mapping.get(project, project)
    graph_file = graph_directory / project / f'{inner_project}-{version}.odem'
    return structure_file, embedding_directory, graph_file


################################################################################
################################################################################
# Feature Utilities
################################################################################


def load_features_for_version(project: str,
                              raw_version: tuple,
                              args: Config,
                              logger: logging.Logger,
                              mapping: dict[str, str]):
    logger.debug('Loading data for %s', raw_version)
    version = '.'.join(map(str, raw_version))
    structure_file, embedding_directory, graph_file = find_files_for_version(
        project,
        version,
        graph_directory=args.graph_directory,
        structure_directory=args.structure_directory,
        embeddings_directory=args.embedding_directory,
        legacy_mapping=mapping
    )
    feature_mapping = build_module_features(
        structure_file, embedding_directory, logger
    )
    with open(graph_file, 'r') as graph_file:
        graph = ODEM.from_xml(graph_file.read())
    return feature_mapping, graph


@torch.no_grad()
def build_module_features(structure_file: pathlib.Path,
                          embedding_directory: pathlib.Path,
                          logger: logging.Logger):
    with open(structure_file) as file:
        structure = json.load(file)
    raw = _build_module_features_recursively(structure, embedding_directory, logger)
    flattened = _flatten_features(raw)
    root = flattened.pop('')
    flattened['@project'] = root
    return flattened


@torch.no_grad()
def _build_module_features_recursively(structure,
                                       embedding_directory: pathlib.Path,
                                       logger: logging.Logger):
    kind = structure.pop('#type')
    if kind == 'Entity':
        path = embedding_directory / structure['path']
        path = path.parent / (path.name + '.bin')
        result = {'@value': torch.load(path)}
        return result
    else:
        result = {}
        assert kind == 'Root' or kind == 'Package', kind
        for name, sub_structure in structure.items():
            result[name] = _build_module_features_recursively(
                sub_structure, embedding_directory, logger
            )
        result['@value'] = torch.mean(
            torch.stack([
                (x['@value'] if isinstance(x, dict) else x)
                for x in result.values()
            ]),
            dim=0
        )
        return result


@torch.no_grad()
def _flatten_features(raw):
    self = raw.pop('@value')
    if not raw:
        return {}   # Make sure classes are excluded
    result = {'': self}
    for key, value in raw.items():
        flattened = _flatten_features(value)
        result |= {
            (f'{key}.{k}' if k else key): v
            for k, v in flattened.items()
        }
    return result


################################################################################
################################################################################
# Multi-version Feature Utilities
################################################################################


@torch.no_grad()
def compute_train_test_data(old, current, new, *,
                            only_shared_packages_in_training=True):
    # For the training we use:
    # 1) Only packages present in both old and current
    # 2) The dependency graph from the _old_ version
    # 3) The features from old
    # 4) The links connected in either old or current as label True
    # 5) The links connected in neither old nor current as label False
    #
    # For testing, we use
    # 1) Only packages present in current and new
    # 2) The dependency graph from current
    # 3) The features from current
    # 4) The connectivity in new as the labels
    training = compute_training_data(
        old, current,
        only_shared_packages=only_shared_packages_in_training
    )
    testing = compute_testing_data(current, new)
    return (
        _as_homogeneous_graph(training[0], training[1]),
        _as_homogeneous_graph(testing[0], testing[1])
    )


def compute_training_data(old, current, *, only_shared_packages=True):
    features_old, graph_old = old
    features_cur, graph_cur = current
    vertices_old, edges_old = _graph_to_list(graph_old)
    vertices_cur, edges_cur = _graph_to_list(graph_cur)
    if only_shared_packages:
        vertices_train = vertices_old
        all_edges_train = {(x, y)
                           for x in vertices_train
                           for y in vertices_train
                           if x != y}
        edges_train = edges_old & all_edges_train
        features_train = {x: features_old[x] for x in vertices_train}
        joint_vertices = vertices_old & vertices_cur
        training_data = (vertices_train, edges_train, features_train, joint_vertices)
        training_labels = all_edges_train & (edges_old | edges_cur)
        return training_data, training_labels
    else:
        # Note that only_shared_packages=False should only
        # change the labels,
        # such that examples from the current graph can
        # also be used  for training.
        # Main question: how to handle the features?
        raise NotImplementedError


def compute_testing_data(current, new):
    features_cur, graph_cur = current
    features_new, graph_new = new
    vertices_cur, edges_cur = _graph_to_list(graph_cur)
    vertices_new, edges_new = _graph_to_list(graph_new)
    vertices_test = vertices_cur
    all_edges_test = {(x, y)
                      for x in vertices_test
                      for y in vertices_test
                      if x != y}
    edges_test = edges_cur & all_edges_test
    features_test = {x: features_cur[x] for x in vertices_test}
    testing_data = (vertices_test, edges_test, features_test, vertices_test)
    testing_labels = all_edges_test & edges_new
    return testing_data, testing_labels


def _graph_to_list(graph: ODEM) -> tuple[set[str], set[tuple[str, str]]]:
    vertices = set()
    edges = set()
    for context in graph.contexts:
        for container in context.containers:
            for namespace in container.namespaces:
                vertices.add(namespace.name)
    for context in graph.contexts:
        for container in context.containers:
            for namespace in container.namespaces:
                for type_ in namespace.types:
                    for dependency in type_.dependencies.depends_on:
                        dependency_package = dependency.name.rsplit(
                            '.', maxsplit=1
                        )[0]
                        if dependency_package in vertices:
                            edges.add((namespace.name, dependency_package))
    return vertices, edges


@torch.no_grad()
def _as_homogeneous_graph(features, labels):
    vertices, edges, feature_mapping, joint_vertices = features
    vertices = list(vertices)
    vertex_mapping = {x: i for i, x in enumerate(vertices)}
    node_features = torch.stack([feature_mapping[x] for x in vertices])
    edges = torch.tensor(
        [
            [vertex_mapping[x[0]] for x in edges],
            [vertex_mapping[x[1]] for x in edges],
        ]
    )

    prediction_edges_unmapped = [
        (x, y)
        for x in joint_vertices
        for y in joint_vertices
        if x != y
    ]
    prediction_edges = [
        (vertex_mapping[x], vertex_mapping[y])
        for x, y in prediction_edges_unmapped
    ]

    label_edges = torch.tensor(prediction_edges)
    labels = torch.tensor([x in labels for x in prediction_edges_unmapped])
    data = torch_geometric.data.Data(
        x=node_features,
        edge_index=edges,
        edge_attr=None,
        y=labels,
    )

    #data = ToUndirected().forward(data)

    return data, label_edges


################################################################################
################################################################################
# Models
################################################################################


class AbstractModel(abc.ABC):

    def __init__(self, logger: logging.Logger):
        self.logger = logger
        self._trained = False

    @abc.abstractmethod
    def train(self, data, eval_connections):
        self._trained = True

    @abc.abstractmethod
    def predict(self, data, eval_connections):
        if not self._trained:
            raise ValueError('Model has not been trained yet.')

    @torch.no_grad()
    def evaluate(self, data, eval_connections):
        if not self._trained:
            raise ValueError('Model has not been trained yet.')
        return evaluate(self.predict(data, eval_connections), data.y)


class GnnWrapper(AbstractModel):

    def __init__(self, model: torch.nn.Module, logger: logging.Logger):
        super().__init__(logger)
        self.model = model

    def train(self, features, connections):
        pass

    def predict(self, features, connections):
        pass



class SvmWrapper(AbstractModel):

    def __init__(self, logger: logging.Logger):
        super().__init__(logger)
        self._svm = SVC(kernel='rbf', cache_size=1000, random_state=42)

    def train(self, features, connections):
        super().train(features, connections)
        with torch.no_grad():
            data = []
            labels = []
            for i, (x, y) in enumerate(connections):
                a = features.x[x, :]
                b = features.x[y, :]
                f = torch.mul(a, b).numpy()
                data.append(f)
                labels.append(features.y[i].item())
            self._svm.fit(numpy.array(data), numpy.array(labels))

    def predict(self, features, connections):
        with torch.no_grad():
            data = []
            for i, (x, y) in enumerate(connections):
                a = features.x[x, :]
                b = features.x[y, :]
                f = torch.mul(a, b).numpy()
                data.append(f)
            pred = self._svm.predict(numpy.array(data))
            return torch.from_numpy(pred).float()


class DummyModel(AbstractModel):

    def train(self, features, connections):
        super().train(features, connections)
        return [
            {
                'train': {
                    **evaluate(self._predict(features, connections), features.y)
                },
                'eval': {}
            }
        ]

    def predict(self, features, connections):
        return self._predict(features, connections)

    def _predict(self, features, connections):
        current = self._current_connections(features)
        # print(current)
        # print(list(connections[0]))
        # raise
        return torch.tensor(
            [(x[0].item(), x[1].item()) in current for x in list(connections)],
            dtype=torch.float
        )

    @staticmethod
    def _current_connections(data: torch_geometric.data.Data):
        return set((x.item(), y.item()) for x, y in data.edge_index.T)


    ################################################################################
################################################################################
# GNN Models
################################################################################


class SimpleGnn(torch.nn.Module):

    def __init__(self):
        super().__init__()

    def forward(self, data):
        pass


################################################################################
################################################################################
# Evaluation
################################################################################


class WeightedAveragePrecision(torchmetrics.Metric):
    pass


def evaluate(predictions, targets):
    shared_args = {
        #'threshold': 0.5,
        'task': 'binary'
    }
    metrics = {
        'accuracy': torchmetrics.Accuracy(**shared_args),
        'precision': torchmetrics.Precision(**shared_args, zero_division=0),
        'recall': torchmetrics.Recall(**shared_args, zero_division=0),
        'f1-score': torchmetrics.F1Score(**shared_args, zero_division=0),
        'aur-roc': torchmetrics.AUROC(task='binary'),
        'kappa': torchmetrics.CohenKappa(**shared_args),
        'matthews-correlation': torchmetrics.MatthewsCorrCoef(**shared_args),
        'specificity': torchmetrics.Specificity(**shared_args, zero_division=0),
        'area-under-precision-recall-curve': torchmetrics.AveragePrecision(**shared_args)
    }
    result = {
        k: v(predictions, targets).item()
        for k, v in metrics.items()
    }
    roc = torchmetrics.ROC(task='binary')
    fpr, tpr, thresholds = roc(predictions, targets)
    result['roc'] = {
        'false-positive-rate': fpr.tolist(),
        'true-positive-rate': tpr.tolist(),
        'thresholds': thresholds.tolist()
    }
    prc = torchmetrics.PrecisionRecallCurve(task='binary')
    precision, recall, thresholds = prc(predictions, targets)
    thresholds = thresholds.tolist()
    if isinstance(thresholds, float):
        thresholds = [thresholds]
    result['prc'] = {
        'precision': precision.tolist(),
        'recall': recall.tolist(),
        'thresholds': thresholds
    }
    conf = torchmetrics.ConfusionMatrix(task='binary')
    mat = conf(predictions, targets)
    result['confusion-matrix'] = {
        'tn': mat[0, 0].item(),
        'fp': mat[0, 1].item(),
        'fn': mat[1, 0].item(),
        'tp': mat[1, 1].item(),
    }
    return result



################################################################################
################################################################################
# Pipeline
################################################################################


def pipeline(args: Config, logger: logging.Logger):
    mapping = dict(
        item.split('=')
        for item in args.project_legacy_mapping.split(';')
    )
    now = datetime.datetime.now().strftime('%Y-%m-%d-%H-%M-%S')
    filename_base = f'{now}__{args.model}'
    for project in os.listdir(args.structure_directory):
        if project in args.exclude_projects:
            logger.info('Skipping project %s', project)
            continue
        logger.info('Processing project %s', project)
        logger.info('Collecting version triples...')
        triples = get_version_triples(args.structure_directory / project)
        logger.info(f'Found {len(triples)} version triples.')
        cache = {}
        for triple in triples:
            logger.debug(' * %s --> %s --> %s', *triple)
            for t in triple:
                if t in cache:
                    continue
                cache[t] = load_features_for_version(
                    project, t, args, logger, mapping
                )
            logger.debug(' * Dropping stale cache entries...')
            cache = {k: v for k, v in cache.items() if k in triple}
            train, test = compute_train_test_data(
                cache[triple[0]], cache[triple[1]], cache[triple[2]]
            )
            match args.model:
                case 'simple-gnn':
                    model = GnnWrapper(SimpleGnn(), logger)
                case 'tommasel-svm':
                    model = SvmWrapper(logger)
                case 'dummy':
                    model = DummyModel(logger)
                case _ as x:
                    raise ValueError(f'Unknown model {x}')
            training_metrics = model.train(*train)
            testing_metrics = model.evaluate(*test)
            v1 = '.'.join(map(str, triple[0]))
            v2 = '.'.join(map(str, triple[1]))
            v3 = '.'.join(map(str, triple[2]))
            filename = f'{filename_base}__{project}__{v1}__{v2}__{v3}.json'
            with open(args.output_directory / filename, 'w') as file:
                json.dump(
                    {
                        'training': training_metrics,
                        'testing': testing_metrics
                    },
                    file,
                    indent=2
                )


################################################################################
################################################################################
# Program Entrypoint
################################################################################


def setup_logging(log_file_path: pathlib.Path) -> logging.Logger:
    logger = logging.getLogger(__name__)
    logger.setLevel(logging.DEBUG)
    formatter = logging.Formatter(
        '[{name}][{asctime}][{levelname:8}]: {message}',
        style='{'
    )
    file_handler = logging.FileHandler(log_file_path, mode='w')
    file_handler.setFormatter(formatter)
    file_handler.setLevel(logging.DEBUG)
    logger.addHandler(file_handler)

    stream_handler = logging.StreamHandler(sys.stdout)
    stream_handler.setFormatter(formatter)
    stream_handler.setLevel(logging.INFO)
    logger.addHandler(stream_handler)

    return logger


def main(args: Config):
    os.makedirs(args.output_directory, exist_ok=True)
    logger = setup_logging(args.output_directory / 'log.txt')
    try:
        pipeline(args, logger)
    except Exception as e:
        logger.critical('=' * 99)
        logger.critical('An error occurred!')
        logger.critical('')
        logger.critical('')
        logger.critical('Error message: %s', e)
        logger.critical('')
        logger.critical('')
        logger.critical('Full traceback:')
        logger.critical('')
        tb = traceback.format_exception(e.__class__, e, e.__traceback__)
        for entry in tb:
            for line in entry.rstrip().splitlines():
                logger.critical(line.rstrip())
        logger.critical('=' * 99)
        raise e


if __name__ == '__main__':
    main(Config().parse_args())
