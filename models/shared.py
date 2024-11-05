################################################################################
################################################################################
# Imports
################################################################################

from __future__ import annotations

import enum
import typing

import pydantic

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
        )


class Node(pydantic.BaseModel):
    name: str
    feature_files: list[str]


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


def evaluate(triple: VersionTriple, predictions: list[bool]):
    true_positives = 0
    false_positives = 0
    false_negatives = 0
    true_negatives = 0
    predicted = []

    labels = triple.test_graph.edge_labels.labels
    edges = triple.test_graph.edge_labels.edges

    for edge, (truth, pred) in zip(edges, zip(labels, predictions)):
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
