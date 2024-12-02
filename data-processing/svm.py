"""
Replication of the SVM for dependency prediction as
described by in [1].

[1] Tommasel, A., & Diaz-Pace, J. A. (2022).
    Identifying emerging smells in software designs based on predicting package dependencies.
    Engineering Applications of Artificial Intelligence, 115, 105209.
     https://doi.org/10.1016/j.engappai.2022.105209
"""

import csv
import json
import math
import pathlib

import tap
from sklearn.svm import SVC

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


def load_feature_file(base_path: pathlib.Path, triple: shared.VersionTriple):
    rename = {'apache-derby': 'db-derby', 'hibernate': 'hibernate-core'}
    rev_rename = {v: k for k, v in rename.items()}
    result = []
    for v in [triple.version_1, triple.version_2, triple.version_3]:
        filename = rename.get(triple.project, triple.project) + f'-{v}.json'
        path = base_path / rev_rename.get(triple.project, triple.project) / filename
        with open(path) as file:
            data = json.load(file)
        edge_features = {}
        for item in data['link-features']:
            key = (item['from'], item['to'])
            value = [_maybe_map(item[x][y]) for x, y in _FEATURE_ORDER]
            edge_features[key] = value
        result.append(edge_features)
    return tuple(result)


def _maybe_map(x):
    if math.isnan(x):
        return 0
    return x


def graph_to_data(graph: shared.Graph, feature_map, *, test=True):
    indices = [0, 1] if not test else [2]
    keep_edges = []
    keep_labels = []
    stream = zip(graph.edge_labels.edges, graph.edge_labels.labels)
    for (source, target), flag in stream:
        key = (
            graph.nodes[source].name,
            graph.nodes[target].name,
        )
        for index in indices:
            if key in feature_map[index]:
                keep_edges.append(feature_map[index][key])
                keep_labels.append(flag)
                break
    return keep_edges, keep_labels


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    feature_file: pathlib.Path
    output_file: pathlib.Path
    balanced: bool = False

    def configure(self) -> None:
        self.add_argument('-i', '--input_files')
        self.add_argument('-f', '--feature_file')
        self.add_argument('-o', '--output_file')
        self.add_argument('-b', '--balanced', action='store_true')


def main(config: Config):
    results = []
    for graph_filename in config.input_files:
        triple = shared.VersionTriple.load_and_check(graph_filename,)
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


if __name__ == '__main__':
    main(Config().parse_args())
