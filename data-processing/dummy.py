import json
import pathlib
import random

import tap

import shared


class DummyModel:

    def __init__(self, *, gnn=False):
        self.graph = {}
        self.gnn = gnn

    def train(self, graph: shared.Graph):
        stream = zip(graph.edge_labels.edges, graph.edge_labels.labels)
        for (source, target), flag in stream:
            key = (graph.nodes[source].name, graph.nodes[target].name)
            self.graph[key] = flag

    def predict(self, graph: shared.Graph) -> list[bool]:
        result = []
        for source, target in graph.edge_labels.edges:
            key = (graph.nodes[source].name, graph.nodes[target].name)
            result.append(self.predict_single(key))
        return result

    def predict_single(self, key: tuple[str, str]) -> bool:
        if key in self.graph:
            return self.graph[key]
        if not self.gnn:
            raise RuntimeError(f'Key {key} not found in graph')
        return False


class PositiveDummyModel(DummyModel):

    def predict_single(self, key: tuple[str, str]) -> bool:
        if key in self.graph:
            return self.graph[key]
        if not self.gnn:
            raise RuntimeError(f'Key {key} not found in graph')
        return True


class RandomDummyModel(DummyModel):

    def predict_single(self, key: tuple[str, str]) -> bool:
        if key in self.graph:
            return self.graph[key]
        if not self.gnn:
            raise RuntimeError(f'Key {key} not found in graph')
        return random.choice([True, False])


class WeightedRandomDummyModel(DummyModel):

    def __init__(self, *, gnn=False):
        super().__init__(gnn=gnn)
        self.weight = None

    def train(self, graph: shared.Graph):
        super().train(graph)
        self.weight = sum(self.graph.values()) / len(self.graph)

    def predict_single(self, key: tuple[str, str]) -> bool:
        if key in self.graph:
            return self.graph[key]
        if not self.gnn:
            raise RuntimeError(f'Key {key} not found in graph')
        return random.random() < self.weight


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    output_file: pathlib.Path
    undirected: bool = False
    gnn: bool = False

    def configure(self) -> None:
        self.add_argument('-i', '--input_files')
        self.add_argument('-o', '--output_file')
        self.add_argument('-u', '--undirected', action='store_true')


def main(config: Config):
    results = {'dummy': []}
    for filename in config.input_files:
        triple = shared.VersionTriple.load_and_check(filename)
        if config.gnn and not triple.metadata.gnn_safe:
            raise ValueError(f'Version triple {triple.project}:{triple.version_1} '
                             f'is not a GNN version triple')
        if config.undirected:
            triple = triple.as_undirected_problem()
        print(f'Loaded version triple from project {triple.project}: '
              f'{triple.version_1}, {triple.version_2}, {triple.version_3}')
        model = DummyModel(gnn=config.gnn)
        model.train(triple.training_graph)
        predictions = model.predict(triple.test_graph)
        result = shared.evaluate(triple, predictions)
        results['dummy'].append(result)
        if config.gnn:
            pos_model = PositiveDummyModel(gnn=True)
            rand_model = RandomDummyModel(gnn=True)
            weighted_rand_model = WeightedRandomDummyModel(gnn=True)
            pos_model.train(triple.training_graph)
            rand_model.train(triple.training_graph)
            weighted_rand_model.train(triple.training_graph)
            pos_predictions = pos_model.predict(triple.test_graph)
            rand_predictions = rand_model.predict(triple.test_graph)
            weighted_rand_predictions = weighted_rand_model.predict(triple.test_graph)
            pos_result = shared.evaluate(triple, pos_predictions)
            rand_result = shared.evaluate(triple, rand_predictions)
            weighted_rand_result = shared.evaluate(triple, weighted_rand_predictions)
            results.setdefault('dummy-positive', []).append(pos_result)
            results.setdefault('dummy-random', []).append(rand_result)
            results.setdefault('dummy-weighted-random', []).append(weighted_rand_result)
    config.output_file.mkdir(parents=True, exist_ok=True)
    for key, value in results.items():
        with open(config.output_file / f'{key}.json', 'w') as file:
            json.dump(value, file, indent=2)


if __name__ == '__main__':
    main(Config().parse_args())
