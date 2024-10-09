################################################################################
################################################################################
# Imports and Constants
################################################################################

import json
import logging
import math
import os
import re
import sys

from sklearn.metrics import accuracy_score, precision_recall_fscore_support
from sklearn.svm import SVC

DATA_DIRECTORY = './data'
RESULT_DIRECTORY = './results'

################################################################################
################################################################################
# File Triple Loading
################################################################################


def get_version_triples():
    for project, versions_for_project in get_versions():
        triples_for_project = []
        for i in range(len(versions_for_project) - 3 + 1):
            triple = versions_for_project[i:i + 3]
            triples_for_project.append(triple)
        yield project, triples_for_project


def get_versions():
    for directory in os.listdir(DATA_DIRECTORY):
        path = os.path.join(DATA_DIRECTORY, directory)
        if not os.path.isdir(path):
            continue
        version_files = [
            file
            for file in os.listdir(path)
            if file.endswith('.json')
        ]
        versions = [extract_version_from_filename(filename)
                    for filename in version_files]
        pairs = list(zip(versions, version_files))
        pairs.sort(key=lambda x: x[0])
        version_pairs = [
            (version, os.path.join(DATA_DIRECTORY, directory, filename))
            for version, filename in pairs
        ]
        yield directory, version_pairs


def extract_version_from_filename(filename):
    pattern = re.compile(r'[a-zA-Z\-_]+(?P<digits>\d+(\.\d+)+)\.')
    result = pattern.match(filename)
    if result is None:
        raise ValueError(f'Filename {filename} does not match pattern')
    return tuple(map(int, result.group('digits').split('.')))


################################################################################
################################################################################
# Feature Loading
################################################################################

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


class ProjectGraph:

    def __init__(self, filename: str):
        with open(filename) as file:
            data = json.load(file)
        self._features = {
            (item['to'], item['from']): [
                x if not math.isnan(x) else 0 for x in
                (item[key_1][key_2] for key_1, key_2 in _FEATURE_ORDER)
            ]
            for item in data['link-features']
        }
        self._edges = {
            (edge['from'], edge['to'])
            for edge in data['edges']
            if (edge['from'], edge['to']) in self._features or (edge['to'], edge['from']) in self._features
        }
        self._nodes = {node for pair in self._edges for node in pair}

    def all_possible_edges(self):
        remainder = self._nodes.copy()
        for x in self._nodes:
            remainder.remove(x)
            for y in remainder:
                if x != y:
                    yield x, y

    def existing_edges(self):
        yield from self._edges

    def feature_for_edge(self, e):
        x, y = e
        try:
            return self._features[(x, y)]
        except KeyError:
            try:
                return self._features[(y, x)]
            except KeyError:
                raise ValueError(f'No edge between {x} and {y}')

    def edges_with_features(self):
        yield from self._features.items()

    def has_edge(self, e):
        x, y = e
        return (x, y) in self._edges or (y, x) in self._edges


################################################################################
################################################################################
# Dataset Preparation
################################################################################


def load_datasets(triple):
    graph_old = ProjectGraph(triple[0][1])
    graph_cur = ProjectGraph(triple[1][1])
    graph_new = ProjectGraph(triple[2][1])
    training_data = build_training_data(graph_old, graph_cur)
    testing_data = build_testing_data(graph_cur, graph_new)
    return training_data, testing_data


def build_training_data(graph_old: ProjectGraph, graph_cur: ProjectGraph):
    features = []
    labels = []
    for edge in graph_old.existing_edges():
        features.append(graph_old.feature_for_edge(edge))
        labels.append(True)
    for edge in graph_cur.existing_edges():
        features.append(graph_cur.feature_for_edge(edge))
        labels.append(True)
    negative = set(graph_old.all_possible_edges()) - set(graph_old.existing_edges())
    negative -= set(graph_cur.existing_edges())
    for edge in negative:
        features.append(graph_old.feature_for_edge(edge))
        labels.append(False)
    return features, labels


def build_testing_data(graph_cur: ProjectGraph, graph_new: ProjectGraph):
    features = []
    labels = []
    for edge, feat in graph_cur.edges_with_features():
        features.append(feat)
        labels.append(graph_new.has_edge(edge))
    return features, labels


################################################################################
################################################################################
# Training and Evaluation
################################################################################


def train_model(features, labels, logger) -> SVC:
    logger.debug('Training model with %s samples', len(features))
    model = SVC(kernel='rbf', cache_size=1000, random_state=42)
    result = model.fit(features, labels)
    assert isinstance(result, SVC)
    return result


def evaluate_model(model, features, labels):
    predictions = model.predict(features)

    precision, recall, f1_score, support = precision_recall_fscore_support(
        labels, predictions, average='binary', zero_division=0
    )
    result = {
        'accuracy': accuracy_score(labels, predictions),
        'precision': precision,
        'recall': recall,
        'f1_score': f1_score,
        'support': support
    }
    return result


################################################################################
################################################################################
# Main Function
################################################################################


def main():
    logger = logging.getLogger(__name__)
    logger.setLevel(logging.DEBUG)

    formatter = logging.Formatter(
        '%(name)s - %(levelname)s - %(asctime)s - %(message)s', style='%'
    )

    handler = logging.StreamHandler(sys.stdout)
    handler.setFormatter(formatter)
    handler.setLevel(logging.DEBUG)
    logger.addHandler(handler)

    handler = logging.FileHandler('logs.txt', mode='w')
    handler.setFormatter(formatter)
    handler.setLevel(logging.DEBUG)
    logger.addHandler(handler)

    for project, triples in get_version_triples():
        logger.info('Processing versions from project %s', project)
        for triple in triples:
            logger.info('Found a version triple: %s, %s, %s',
                         triple[0][0], triple[1][0], triple[2][0])
            for version, filename in triple:
                logger.debug('File: %s --> %s', version, filename)
            logger.info('Loading features and labels...')
            train, test = load_datasets(triple)
            if len(set(train[1])) == 1:
                logger.warning('Skipping triple since the dataset contains only one label.')
                continue
            logger.info('Training model...')
            model = train_model(*train, logger=logger)
            logger.info('Evaluating model...')
            metrics = evaluate_model(model, *test)
            logger.info('Accuracy: %s', metrics['accuracy'])
            logger.info('Precision: %s', metrics['precision'])
            logger.info('Recall: %s', metrics['recall'])
            logger.info('F1 Score: %s', metrics['f1_score'])
            logger.info('Saving metrics...')
            os.makedirs(os.path.join(RESULT_DIRECTORY, project), exist_ok=True)
            filename = os.path.join(
                RESULT_DIRECTORY,
                project,
                f'{triple[0][0]}__{triple[1][0]}__{triple[2][0]}.json'
            )
            with open(filename, 'w') as file:
                json.dump(metrics, file, indent=2)


if __name__ == '__main__':
    main()