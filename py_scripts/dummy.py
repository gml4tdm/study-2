import json
import pathlib

import tap

import shared


class DummyModel:

    def __init__(self):
        self.graph = {}

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
        return self.graph[(key[1], key[0])]


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    output_file: pathlib.Path
    undirected: bool = False

    def configure(self) -> None:
        self.add_argument('-i', '--input_files')
        self.add_argument('-o', '--output_file')
        self.add_argument('-u', '--undirected', action='store_true')


def main(config: Config):
    results = []
    for filename in config.input_files:
        triple = shared.VersionTriple.load_and_check(filename)
        if config.undirected:
            triple = triple.as_undirected_problem()
        print(f'Loaded version triple from project {triple.project}: '
              f'{triple.version_1}, {triple.version_2}, {triple.version_3}')
        model = DummyModel()
        model.train(triple.training_graph)
        predictions = model.predict(triple.test_graph)
        result = shared.evaluate(triple, predictions)
        results.append(result)
    config.output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(config.output_file, 'w') as file:
        json.dump(results, file, indent=2)


if __name__ == '__main__':
    main(Config().parse_args())
