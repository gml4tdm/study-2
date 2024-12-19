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

import shared


_FEATURE_ORDER = [
    ('topological-features', 'common_neighbours'),
    ('topological-features', 'salton'),
    ('topological-features', 'sorensen'),
    ('topological-features', 'adamic_adar'),
    ('topological-features', 'katz'),
    ('topological-features', 'sim_rank'),
    ('topological-features', 'russel_rao'),
    ('topological-features', 'resource_allocation'),
    ('semantic-features', 'comments#Cosine'),
    ('semantic-features', 'imports#Cosine'),
    ('semantic-features', 'methods#Cosine'),
    ('semantic-features', 'variables#Cosine'),
    ('semantic-features', 'fields#Cosine'),
    ('semantic-features', 'calls#Cosine'),
    ('semantic-features', 'imports-fields-methods-variables-comments#Cosine'),
    ('semantic-features', 'imports-fields-methods-variables#Cosine'),
    ('semantic-features', 'fields-variables-methods#Cosine'),
    ('semantic-features', 'fields-methods#Cosine'),
    ('semantic-features', 'fields-variables#Cosine'),
    ('semantic-features', 'imports-fields-methods-variables-comments-calls#Cosine'),
    ('semantic-features', 'imports-fields-methods-variables-calls#Cosine'),
    ('semantic-features', 'fields-variables-methods-calls#Cosine'),
    ('semantic-features', 'fields-methods-calls#Cosine'),
    ('semantic-features', 'methods-calls#Cosine')
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
        return self.link_changes[self._inverse_link_map[(fr, to)]]


class LinkChangeInfo(pydantic.BaseModel):
    additions: int
    deletions: int
    was_new: bool
    was_removed: bool


class NodeChangeInfo(pydantic.BaseModel):
    added_incoming: int
    added_outgoing: int
    removed_incoming: int
    removed_outgoing: int
    was_new: bool
    was_removed: bool
    added_classes: bool
    removed_classes: bool


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


class PairedCoChangeData(pydantic.BaseModel):
    lifetime_change_likelihood: float
    version_change_likelihood: float


class UnitCoChangeData:
    time_since_last_change: float
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
    links_without_semantic_features: list[GraphEdge] = pydantic.Field(
        alias='links-without-semantic-features'
    )
    link_without_topology: list[GraphEdge] = pydantic.Field(
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
    topological_features: list[float] = pydantic.Field(
        alias='topological-features'
    )
    semantic_features: list[float] = pydantic.Field(alias='semantic-features')


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
                    getattr(features, ns)[key]
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
            unit_fr.time_since_last_change,
            unit_fr.version_co_change_prospect,
            unit_fr.lifetime_co_change_prospect,
            unit_to.time_since_last_change,
            unit_to.version_co_change_prospect,
            unit_to.lifetime_co_change_prospect,
            paired.version_change_likelihood,
            paired.lifetime_change_likelihood,
        ]


def _add_graph_change_features_to_dataset(dataset, graph_changes: GraphChangeData):
    for fr, to in dataset:
        node_fr = graph_changes.node_changes[fr]
        node_to = graph_changes.node_changes[to]
        link = graph_changes.get_link_changes(fr, to)
        dataset[(fr, to)]['features']['graph_change'] = [
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

    def configure(self) -> None:
        self.add_argument('-t', '--triple_files')
        self.add_argument('-g', '--graph_change_files')
        self.add_argument('-c', '--co_change_files')
        self.add_argument('-m', '--metric_files')
        self.add_argument('-o', '--output_file')


def dissect_triple_filename(filename: pathlib.Path) -> tuple[str, str, str, str]:
    res = tuple(filename.stem.split('-'))
    assert len(res) == 4
    return res      # type: ignore


def dissect_metrics_filename(filename: pathlib.Path) -> tuple[str, str]:
    res = tuple(filename.stem.split('-'))
    assert len(res) == 2
    return res      # type: ignore


def dissect_co_change_filename(filename: pathlib.Path) -> str:
    return filename.stem


def dissect_graph_change_filename(filename: pathlib.Path) -> str:
    return filename.stem


def get_major_and_minor(v: str) -> tuple[str, str]:
    parts = v.split('.')
    return parts[0], parts[1]


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
            v1_major, v1_minor = get_major_and_minor(version_1)
            v2_major, v2_minor = get_major_and_minor(version_2)
            co_changes_v1 = co_change_data.root[v1_major][v1_minor]
            co_changes_v2 = co_change_data.root[v2_major][v2_minor]
            graph_changes_v1 = graph_change_data.get_changes_for_graph(version_1)
            graph_changes_v2 = graph_change_data.get_changes_for_graph(version_2)
            metrics_v1 = MetricsDataSet.load(
                files['metrics'][version_1]
            )
            metrics_v2 = MetricsDataSet.load(
                files['metrics'][version_2]
            )
            features_train, labels_train, _ = get_dataset(
                triple_data.training_graph,
                metrics_v1,
                co_changes_v1,
                graph_changes_v1
            )
            features_test, labels_test, test_edges = get_dataset(
                triple_data.test_graph,
                metrics_v2,
                co_changes_v2,
                graph_changes_v2
            )
            model_parameters = dict(kernel='rbf',
                                    cache_size=1999,
                                    random_state=42,
                                    gamma=0.01)
            model = SVC(**model_parameters)
            model.fit(features_train, labels_train)
            predictions = model.predict(features_test).tolist()
            result = shared.evaluate(triple_data,
                                     predictions,
                                     limited_to=test_edges)
            results.append(result)

    config.output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(config.output_file, 'w') as file:
        json.dump(results, file, indent=2)


if __name__ == "__main__":
    main(Config().parse_args())
