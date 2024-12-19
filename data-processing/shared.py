################################################################################
################################################################################
# Imports
################################################################################

from __future__ import annotations

import enum
import os
import pathlib
import re
import typing

import pydantic
import torch


################################################################################
################################################################################
# Version Triple Schema
################################################################################

class VersionTriple(pydantic.BaseModel):
    project: str
    version_1: str
    version_2: str
    version_3: str
    training_graph: Graph
    test_graph: Graph
    metadata: VersionTripleMetadata

    @classmethod
    def load_and_check(cls, filename) -> typing.Self:
        with open(filename, 'r') as file:
            self = pydantic.TypeAdapter(cls).validate_json(file.read())
        if self.metadata.magic_number != 0x00_01_01_01:
            raise ValueError(f'Invalid magic number: {self.metadata.magic_number}')
        return self

    def as_undirected_problem(self) -> typing.Self:
        return VersionTriple(
            project=self.project,
            version_1=self.version_1,
            version_2=self.version_2,
            version_3=self.version_3,
            training_graph=self.training_graph.as_undirected_problem(),
            test_graph=self.test_graph.as_undirected_problem(),
            metadata=self.metadata,
        )


class VersionTripleMetadata(pydantic.BaseModel):
    only_common_nodes_for_training: bool
    gnn_safe: bool
    magic_number: int


class Graph(pydantic.BaseModel):
    nodes: list[Node]
    edges: list[Edge]
    hierarchies: list[NodeHierarchy]
    edge_labels: EdgeLabels
    directed: bool
    classes: list[Class]

    def as_undirected_problem(self) -> typing.Self:
        if not self.directed:
            return self
        undirected_edges = []
        seen = set()
        for edge in self.edges:
            key = (edge.from_, edge.to)
            rev_key = (edge.to, edge.from_)
            if key not in seen and rev_key not in seen:
                undirected_edges.append(edge.as_undirected_problem())
                seen.add(key)
        return Graph(
            nodes=self.nodes,
            edges=undirected_edges,
            hierarchies=self.hierarchies,
            edge_labels=self.edge_labels.as_undirected_problem(),
            directed=False,
            classes=self.classes,
        )


class Class(pydantic.BaseModel):
    package: str
    name: str
    versions: list[int]


class Node(pydantic.BaseModel):
    name: str
    versions: list[int]
    files: dict[str, str]


class Edge(pydantic.BaseModel):
    from_: int = pydantic.Field(alias='from')
    to: int
    edge_type: DependencySpec

    def as_undirected_problem(self) -> typing.Self:
        return Edge(**{
            'from': self.to,
            'to': self.from_,
            'edge_type': self.edge_type
        })


class NodeHierarchy(pydantic.BaseModel):
    name: str
    index: int | None
    children: list[NodeHierarchy]
    versions: list[int]


class EdgeLabels(pydantic.BaseModel):
    edges: list[tuple[int, int]]
    labels: list[bool]

    def as_undirected_problem(self) -> typing.Self:
        lookup = {
            (fr, to): flag
            for (fr, to), flag in zip(self.edges, self.labels)
        }
        undirected_edges = []
        seen = set()
        undirected_labels = []
        for (fr, to), flag in zip(self.edges, self.labels):
            if (fr, to) not in seen and (to, fr) not in seen:
                undirected_edges.append((fr, to))
                seen.add((fr, to))
                if not flag:
                    flag = flag or lookup.get((to, fr))
                undirected_labels.append(flag)
        return EdgeLabels(
            edges=undirected_edges,
            labels=undirected_labels,
        )


class DependencySpec(pydantic.BaseModel):
    counts: dict[DependencyType, int]


class DependencyType(str, enum.Enum):
    Uses = 'Uses'
    Extends = 'Extends'
    Implements = 'Implements'
    Unspecified = 'Unspecified'

################################################################################
################################################################################
# Evaluation
################################################################################



def evaluate(triple: VersionTriple,
             predictions: list[bool], *,
             limited_to: set[str] | None = None):
    true_positives = 0
    false_positives = 0
    false_negatives = 0
    true_negatives = 0
    predicted = []

    labels = triple.test_graph.edge_labels.labels
    edges = triple.test_graph.edge_labels.edges

    for edge, (truth, pred) in zip(edges, zip(labels, predictions)):
        if limited_to is not None and edge not in limited_to:
            continue
        if truth and pred:
            true_positives += 1
        elif truth and not pred:
            false_negatives += 1
        elif pred and not truth:
            false_positives += 1
        elif not truth and not pred:
            true_negatives += 1
        if pred:
            predicted.append(
                (
                    triple.test_graph.nodes[edge[0]].name,
                    triple.test_graph.nodes[edge[1]].name
                )
            )

    return {
        'project': triple.project,
        'version_1': triple.version_1,
        'version_2': triple.version_2,
        'version_3': triple.version_3,
        'output': {
            'true_positives': true_positives,
            'false_positives': false_positives,
            'false_negatives': false_negatives,
            'true_negatives': true_negatives,
            'predicted_dependencies': predicted,
        }
    }


################################################################################
################################################################################
# Package Features
################################################################################


def build_node_features(graph: Graph,
                        source_directory: pathlib.Path,
                        embedding_directory: pathlib.Path):
    feature_map = {}
    file_mapping = scan_source_directory(source_directory)
    for hierarchy in graph.hierarchies:
        build_node_features_recursively(
            hierarchy, feature_map, file_mapping, graph, embedding_directory
        )
    features = [feature_map[k] for k in sorted(feature_map.keys())]
    return torch.stack(features)


def build_node_features_recursively(
        hierarchy: NodeHierarchy,
        feature_map: dict[int, torch.Tensor],
        file_mapping: dict[str, list[pathlib.Path]],
        graph: Graph,
        embedding_directory: pathlib.Path):
    if hierarchy.index is None:
        for child in hierarchy.children:
            build_node_features_recursively(
                child, feature_map, file_mapping, graph, embedding_directory
            )
    elif hierarchy.index not in feature_map:
        for child in hierarchy.children:
            build_node_features_recursively(
                child, feature_map, file_mapping, graph, embedding_directory
            )
        child_features = [
            feature_map[child.index]
            for child in hierarchy.children
        ]
        class_features = [
            load_embedding(filename, embedding_directory)
            for  filename in file_mapping[hierarchy.name]
        ]
        feature_map[hierarchy.index] = torch.mean(
            torch.stack(child_features + class_features),
            dim=0,
        )


def scan_source_directory(path: pathlib.Path):
    file_mapping = {}
    scan_source_directory_recursive(path, path, file_mapping)
    return file_mapping


def scan_source_directory_recursive(path: pathlib.Path, root: pathlib.Path, file_mapping):
    for filename in os.listdir(path):
        file_path = path / filename
        if file_path.is_dir():
            scan_source_directory_recursive(file_path, root, file_mapping)
        elif file_path.is_file():
            package = scan_source_file(file_path)
            file_mapping.setdefault(package, []).append(file_path.relative_to(root))


def scan_source_file(path: pathlib.Path):
    pattern = re.compile(r'^\s*package\s+(?P<package>[a-zA-Z0-9_.]+);')
    with open(path, encoding='utf-8', errors='ignore') as file:
        for line in file:
            if (m := pattern.match(line)) is not None:
                return m.group('package')
    raise ValueError(f'Could not find package in {path}')


def load_embedding(path: pathlib.Path, embedding_directory: pathlib.Path):
    path = embedding_directory / path.with_suffix(f'{path.suffix}.bin')
    print(f'Loading {path}')
    return torch.load(path)
