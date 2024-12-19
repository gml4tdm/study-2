################################################################################
################################################################################
# Imports
################################################################################

from __future__ import annotations

import functools
import json
import math
import pathlib
import typing

import numpy

import tap
from sklearn.svm import SVC
import pydantic
import pydantic.dataclasses

import shared


_FEATURE_ORDER = [
    ('topological_features', 'common_neighbours'),
    ('topological_features', 'salton'),
    ('topological_features', 'sorensen'),
    ('topological_features', 'adamic_adar'),
    ('topological_features', 'katz'),
    ('topological_features', 'sim_rank'),
    ('topological_features', 'russel_rao'),
    ('topological_features', 'resource_allocation'),
    ('semantic_features', 'comments#Cosine'),
    ('semantic_features', 'imports#Cosine'),
    ('semantic_features', 'methods#Cosine'),
    ('semantic_features', 'variables#Cosine'),
    ('semantic_features', 'fields#Cosine'),
    ('semantic_features', 'calls#Cosine'),
    ('semantic_features', 'imports-fields-methods-variables-comments#Cosine'),
    ('semantic_features', 'imports-fields-methods-variables#Cosine'),
    ('semantic_features', 'fields-variables-methods#Cosine'),
    ('semantic_features', 'fields-methods#Cosine'),
    ('semantic_features', 'fields-variables#Cosine'),
    ('semantic_features', 'imports-fields-methods-variables-comments-calls#Cosine'),
    ('semantic_features', 'imports-fields-methods-variables-calls#Cosine'),
    ('semantic_features', 'fields-variables-methods-calls#Cosine'),
    ('semantic_features', 'fields-methods-calls#Cosine'),
    ('semantic_features', 'methods-calls#Cosine')
]


################################################################################
################################################################################
# Private Data Schema -- Graph Changes
################################################################################


class GraphChangeDataset(pydantic.BaseModel):
    versions: list[GraphChangeData]

    @classmethod
    def load(cls, filename: pathlib.Path) -> typing.Self:
        with open(filename, 'r') as file:
            return pydantic.TypeAdapter(cls).validate_json(file.read())

    @functools.cached_property
    def _version_map(self):
        return {version.version: version for version in self.versions}

    def get_changes_for_graph(self, v: str):
        return self._version_map[v]


class GraphChangeData(pydantic.BaseModel):
    version: str
    links: dict[str, tuple[str, str]]
    link_changes: dict[str, LinkChangeInfo]
    node_changes: dict[str, NodeChangeInfo]

    @functools.cached_property
    def _inverse_link_map(self):
        return {v: k for k, v in self.links.items()}

    def get_link_changes(self, fr: str, to: str):
        try:
            return self.link_changes[self._inverse_link_map[(fr, to)]]
        except KeyError as e:
            print(self._inverse_link_map)
            print((fr, to))
            raise e

@pydantic.dataclasses.dataclass(frozen=True, slots=True)
class LinkChangeInfo:
    additions: int
    deletions: int
    was_new: bool
    was_removed: bool


@pydantic.dataclasses.dataclass(frozen=True, slots=True)
class NodeChangeInfo:
    added_incoming: int
    added_outgoing: int
    removed_incoming: int
    removed_outgoing: int
    was_new: bool
    was_removed: bool
    added_classes: int
    removed_classes: int


################################################################################
################################################################################
# Private Data Schema -- Co-change Information
################################################################################


class CoChangeData(pydantic.BaseModel):
    old: str
    new: str
    pairs: dict[str, tuple[str, str]]
    paired_features: dict[str, PairedCoChangeData]
    unit_features: dict[str, UnitCoChangeData]

    @functools.cached_property
    def _inverse_link_map(self):
        return {v: k for k, v in self.pairs.items()}

    def get_paired_features(self, fr: str, to: str):
        return self.paired_features[self._inverse_link_map[(fr, to)]]


@pydantic.dataclasses.dataclass(frozen=True, slots=True)
class PairedCoChangeData:
    lifetime_change_likelihood: float
    version_change_likelihood: float


@pydantic.dataclasses.dataclass(frozen=True, slots=True)
class UnitCoChangeData:
    time_since_last_change: float | None        # Very rarely null, no clue why
    lifetime_co_change_prospect: float
    version_co_change_prospect: float


CoChangeDataSet = pydantic.RootModel[dict[str, dict[str, CoChangeData]]]


def load_co_change_data(path: pathlib.Path) -> CoChangeDataSet:
    with open(path, 'r') as file:
        return pydantic.TypeAdapter(CoChangeDataSet).validate_json(file.read())


################################################################################
################################################################################
# Private Data Schema -- Features From Tommasel and Diaz-Pace
################################################################################


class MetricsDataSet(pydantic.BaseModel):
    nodes: list[str]
    edges: list[GraphEdge]
    link_features: list[LinkFeatureData] = pydantic.Field(
        alias='link-features'
    )
    links_without_semantic_features: list[GraphEdge] | None = pydantic.Field(
        alias='links-without-semantics'
    )
    link_without_topology: list[tuple[str, str]] | None = pydantic.Field(
        alias='links-without-topology'
    )

    @classmethod
    def load(cls, filename: pathlib.Path) -> typing.Self:
        with open(filename, 'r') as file:
            return pydantic.TypeAdapter(cls).validate_json(file.read())

    @functools.cached_property
    def _feature_map(self):
        return {(feat.from_, feat.to): feat for feat in self.link_features}

    def get_features_for_edge(self, fr: str, to: str):
        return self._feature_map[(fr, to)]

    def has_data_for(self, fr: str, to: str) -> bool:
        return (fr, to) in self._feature_map


class GraphEdge(pydantic.BaseModel):
    from_: str = pydantic.Field(alias='from')
    to: str


class LinkFeatureData(pydantic.BaseModel):
    from_: str = pydantic.Field(alias='from')
    to: str
    topological_features: dict[str, float] = pydantic.Field(
        alias='topological-features'
    )
    semantic_features: dict[str, float] = pydantic.Field(
        alias='semantic-features'
    )


################################################################################
################################################################################
# Data Preparation
################################################################################


def get_dataset(graph: shared.Graph,
                        metric_data: MetricsDataSet,
                        co_change_data: CoChangeData,
                        graph_changes: GraphChangeData):
    dataset, kept = _build_dataset_base_from_metrics(graph, metric_data)
    _add_co_change_features_to_dataset(dataset, co_change_data)
    _add_graph_change_features_to_dataset(dataset, graph_changes)
    feat, labels = _dict_dataset_to_numpy(dataset)
    return feat, labels, kept


def _dict_dataset_to_numpy(dataset):
    features = []
    labels = []
    for v in dataset.values():
        labels.append(v['label'])
        feat = v['features']
        features.append(
            feat['metrics'] + feat['co_change'] + feat['graph_changes']
        )
    return numpy.asarray(features), numpy.asarray(labels)


def _build_dataset_base_from_metrics(graph: shared.Graph,
                                     metric_data: MetricsDataSet):
    dataset = {}
    kept = set()
    for edge, label in zip(graph.edge_labels.edges, graph.edge_labels.labels):
        fr = graph.nodes[edge[0]].name
        to = graph.nodes[edge[1]].name
        if not metric_data.has_data_for(fr, to):
            continue
        kept.add(edge)
        features = metric_data.get_features_for_edge(fr, to)
        dataset[(fr, to)] = {
            'label': label,
            'features': {
                'metrics': [
                    _maybe_map(getattr(features, ns)[key])
                    for ns, key in _FEATURE_ORDER
                ]
            }
        }
    return dataset, kept


def _add_co_change_features_to_dataset(dataset, co_change_data: CoChangeData):
    for fr, to in dataset:
        unit_fr = co_change_data.unit_features[fr]
        unit_to = co_change_data.unit_features[to]
        paired = co_change_data.get_paired_features(fr, to)
        dataset[(fr, to)]['features']['co_change'] = [
            _maybe_map_none(unit_fr.time_since_last_change),
            _maybe_map(unit_fr.version_co_change_prospect),
            _maybe_map(unit_fr.lifetime_co_change_prospect),
            _maybe_map_none(unit_to.time_since_last_change),
            _maybe_map(unit_to.version_co_change_prospect),
            _maybe_map(unit_to.lifetime_co_change_prospect),
            _maybe_map(paired.version_change_likelihood),
            _maybe_map(paired.lifetime_change_likelihood),
        ]


def _add_graph_change_features_to_dataset(dataset, graph_changes: GraphChangeData):
    for fr, to in dataset:
        node_fr = graph_changes.node_changes[fr]
        node_to = graph_changes.node_changes[to]
        link = graph_changes.get_link_changes(fr, to)
        dataset[(fr, to)]['features']['graph_changes'] = [
            node_fr.added_classes,
            node_fr.removed_classes,
            node_fr.added_incoming,
            node_fr.removed_incoming,
            node_fr.added_outgoing,
            node_fr.removed_outgoing,
            node_fr.was_new,
            node_fr.was_removed,
            node_to.added_classes,
            node_to.removed_classes,
            node_to.added_incoming,
            node_to.removed_incoming,
            node_to.added_outgoing,
            node_to.removed_outgoing,
            node_to.was_new,
            node_to.was_removed,
            link.additions,
            link.deletions,
            link.was_new,
            link.was_removed
        ]


def _maybe_map(x):
    if math.isnan(x):
        return 0
    return x


def _maybe_map_none(x):
    if x is None:
        return 0.0
    return _maybe_map(x)


################################################################################
################################################################################
# Main Function
################################################################################


class Config(tap.Tap):
    triple_files: list[pathlib.Path]
    graph_change_files: list[pathlib.Path]
    co_change_files: list[pathlib.Path]
    metric_files: list[pathlib.Path]
    output_file: pathlib.Path
    limit_to: list[str] = None

    def configure(self) -> None:
        self.add_argument('-t', '--triple_files')
        self.add_argument('-g', '--graph_change_files')
        self.add_argument('-c', '--co_change_files')
        self.add_argument('-m', '--metric_files')
        self.add_argument('-o', '--output_file')
        self.add_argument('-l', '--limit_to', nargs='+')


def dissect_triple_filename(filename: pathlib.Path) -> tuple[str, str, str, str]:
    res = tuple(filename.stem.rsplit('-', maxsplit=3))
    assert len(res) == 4, (filename.stem, res)
    return res      # type: ignore


def dissect_metrics_filename(filename: pathlib.Path) -> tuple[str, str]:
    res = tuple(filename.stem.rsplit('-', maxsplit=1))
    assert len(res) == 2
    return res      # type: ignore


def dissect_co_change_filename(filename: pathlib.Path) -> str:
    return filename.stem


def dissect_graph_change_filename(filename: pathlib.Path) -> str:
    return filename.stem


def get_major_and_minor(v: str) -> tuple[str, str]:
    parts = v.split('.')
    return parts[0], parts[1]


def data_for_version(graph: shared.Graph,
                     co_change_data: CoChangeDataSet,
                     graph_change_data: GraphChangeDataset,
                     files,
                     version: str):
    major, minor = get_major_and_minor(version)
    if minor not in co_change_data.root[major]:
        print('WARNING: Skipping triple. Maybe it is the first in the sequence and thus has no historical data?')
        return None

    co_changes = co_change_data.root[major][minor]
    graph_changes = graph_change_data.get_changes_for_graph(version)
    metrics = MetricsDataSet.load(
        files['metrics'][version]
    )

    return get_dataset(
        graph,
        metrics,
        co_changes,
        graph_changes
    )


def main(config: Config):
    files_by_project = {}

    for filename in config.triple_files:
        project, version_1, version_2, version_3 = dissect_triple_filename(filename)
        files_by_project.setdefault(project, {}).setdefault('triples', []).append(
            (filename, version_1, version_2, version_3)
        )

    for filename in config.graph_change_files:
        project = dissect_graph_change_filename(filename)
        files_by_project.setdefault(project, {})['graph_changes'] = filename

    for filename in config.co_change_files:
        project = dissect_co_change_filename(filename)
        files_by_project.setdefault(project, {})['co_changes'] = filename

    for filename in config.metric_files:
        project, version = dissect_metrics_filename(filename)
        files_by_project.setdefault(project, {}).setdefault('metrics', {})[version] = filename


    results = []
    for project, files in files_by_project.items():
        if config.limit_to is not None and project not in config.limit_to:
            print(f'Skipping project {project}')
            continue

        print(f'Project: {project}')
        co_change_data = load_co_change_data(
            files['co_changes']
        )
        graph_change_data = GraphChangeDataset.load(
            files['graph_changes']
        )

        for triple in files['triples']:
            filename, version_1, version_2, version_3 = triple
            print(f'  Triple: {filename}')
            print(f'    Version 1: {version_1}')
            print(f'    Version 2: {version_2}')
            print(f'    Version 3: {version_3}')
            triple_data = shared.VersionTriple.load_and_check(filename)
            res = data_for_version(
                triple_data.training_graph, co_change_data, graph_change_data, files, version_1
            )
            if res is None:
                continue
            features_train, labels_train, _ = res
            model_parameters = dict(kernel='rbf',
                                    cache_size=1999,
                                    random_state=42,
                                    gamma=0.01)
            model = SVC(**model_parameters)
            model.fit(features_train, labels_train)

            # Make sure memory is re-claimed
            del features_train
            del labels_train

            res = data_for_version(
                triple_data.training_graph, co_change_data, graph_change_data, files, version_1
            )
            if res is None:
                raise ValueError('Test set undefined!')
            features_test, labels_test, test_edges = res
            predictions = model.predict(features_test).tolist()

            del features_test
            del labels_test

            result = shared.evaluate(triple_data,
                                     predictions,
                                     limited_to=test_edges)

            del test_edges

            results.append(result)

    config.output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(config.output_file, 'w') as file:
        json.dump(results, file, indent=2)


if __name__ == "__main__":
    main(Config().parse_args())
