################################################################################
################################################################################
# Imports
################################################################################

from __future__ import annotations

import json
import pathlib
import typing

import torch
import torch_geometric

import tap
from sklearn.svm import SVC
import pydantic

import shared

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


class GraphChangeData(pydantic.BaseModel):
    version: str
    links: dict[str, tuple[str, str]]
    link_changes: dict[str, LinkChangeInfo]
    node_changes: dict[str, NodeChangeInfo]


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


class CoChangeVersion(pydantic.BaseModel):
    old_version: str
    new_version: str
    changes: CoChangeData


class CoChangeData(pydantic.BaseModel):
    changes: dict[str, ChangeInfo]
    co_changes: dict[str, ChangeInfo]
    pairs: dict[str, tuple[str, str]]


class ChangeInfo(pydantic.BaseModel):
    seq: int
    author_date_ts: int
    committer_date_ts: int


CoChangeDataset = pydantic.RootModel[dict[str, dict[str, CoChangeVersion]]]

################################################################################
################################################################################
# Data Preparation
################################################################################


def get_pytorch_dataset(triple: shared.VersionTriple,
                        graph_changes_mapping: None,
                        co_change_mapping: None):
    if not triple.metadata.gnn_safe:
        raise ValueError(f'Triple ({triple.version_1}, {triple.version_2}, {triple.version_3}) is not GNN safe')
    return torch_geometric.data.Data(
        x=feat,
        edge_index=torch.tensor([
            [edge.from_ for edge in graph.edges],
            [edge.to for edge in graph.edges]
        ]),
        pred_edges=torch.tensor(graph.edge_labels.edges),
        y=torch.tensor(graph.edge_labels.labels, dtype=torch.float)
    )


################################################################################
################################################################################
# Main Function
################################################################################


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    source_directory: pathlib.Path
    embedding_directory: pathlib.Path
    output_file: pathlib.Path


def main(config: Config):
    results = []
    for graph_filename in config.input_files:
        triple = shared.VersionTriple.load_and_check(graph_filename)

        feature_map = load_feature_file(config.feature_file, triple)
        print(f'Loaded version triple from project {triple.project}: '
              f'{triple.version_1}, {triple.version_2}, {triple.version_3}')

        model_parameters = dict(kernel='rbf',
                                cache_size=1999,
                                random_state=42,
                                gamma=0.01)
        if config.balanced:
            model_parameters['class_weight'] = 'balanced'
        model = SVC(**model_parameters)
        features, labels = graph_to_data(triple.training_graph, feature_map, test=False)
        model.fit(features, labels)
        features, labels = graph_to_data(triple.test_graph, feature_map, test=True)
        predictions = model.predict(features).tolist()

        result = shared.evaluate(triple, predictions)
        results.append(result)
    config.output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(config.output_file, 'w') as file:
        json.dump(results, file, indent=2)


if __name__ == "__main__":
    main(Config().parse_args())
