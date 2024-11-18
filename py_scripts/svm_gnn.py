################################################################################
################################################################################
# Imports
################################################################################
import json
import os
import pathlib
import re
from xml.dom.domreg import well_known_implementations

import torch
import torch_geometric

import tap
from sklearn.svm import SVC

import shared

################################################################################
################################################################################
# Constants
################################################################################

SOURCE_DIRECTORY = {
    # Apache Ant
    ('apache-ant', '1.1'): 'src/main',
    ('apache-ant', '1.2'): 'src/main',
    ('apache-ant', '1.3'): 'src/main',
    ('apache-ant', '1.4'): 'src/main',
    ('apache-ant', '1.5'): 'src/main',
    ('apache-ant', '1.5.2'): 'src/main',
    ('apache-ant', '1.6.0'): 'src/main',
    ('apache-ant', '1.7.0'): 'src/main',
    ('apache-ant', '1.8.0'): 'src/main',
    ('apache-ant', '1.9.0'): 'src/main',
    ('apache-ant', '1.10.0'): 'src/main',
    # Apache Camel
    ('apache-camel', '2.0.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.1.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.2.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.3.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.4.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.5.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.6.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.7.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.8.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.9.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.10.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.11.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.12.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.13.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.14.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.15.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.16.0'): 'camel-core/src/main/java',
    ('apache-camel', '2.17.0'): 'camel-core/src/main/java',
}

################################################################################
################################################################################
# Data Preparation
################################################################################


def get_pytorch_dataset(graph: shared.Graph,
                        source_directory: pathlib.Path,
                        embedding_directory: pathlib.Path):
    feat = shared.build_node_features(graph, source_directory, embedding_directory)
    return torch_geometric.data.Data(
        x=feat,
        edge_index=torch.tensor([
            [edge.from_ for edge in graph.edges],
            [edge.to for edge in graph.edges]
        ]),
        pred_edges=torch.tensor(graph.edge_labels.edges),
        y=torch.tensor(graph.edge_labels.labels, dtype=torch.float)
    )


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    source_directory: pathlib.Path
    embedding_directory: pathlib.Path
    output_file: pathlib.Path


def main(config: Config):
    results = []
    for filename in config.input_files:
        triple = shared.VersionTriple.load_and_check(filename)
        print(f'Loaded version triple from project {triple.project}: '
              f'{triple.version_1}, {triple.version_2}, {triple.version_3}')
        if not triple.metadata.gnn_safe:
            raise ValueError('Data not prepared for GNN')
        key_1 = (triple.project, triple.version_1)
        key_2 = (triple.project, triple.version_2)
        key_3 = (triple.project, triple.version_3)
        if any(key not in SOURCE_DIRECTORY for key in [key_1, key_2, key_3]):
            raise ValueError(f'No source directory found for {triple.project}')
        train = get_pytorch_dataset(
            triple.training_graph,
            config.source_directory / triple.project / triple.version_1 / SOURCE_DIRECTORY[key_1],
            config.embedding_directory / triple.project / triple.version_1 / SOURCE_DIRECTORY[key_1],
        )
        #t = train.x[train.pred_edges]
        training_data = torch.concat((train.x[train.pred_edges[:, 0]], train.x[train.pred_edges[:, 1]]), dim=1)
        model = SVC(kernel='rbf', cache_size=1999)
        print(training_data.shape)
        print(train.y.shape)
        model.fit(training_data.cpu().detach().numpy(), train.y.cpu().detach().numpy())

        # Evaluation
        del train
        test = get_pytorch_dataset(
            triple.test_graph,
            config.source_directory / triple.project / triple.version_2 / SOURCE_DIRECTORY[key_2],
            config.embedding_directory / triple.project / triple.version_2 / SOURCE_DIRECTORY[key_2],
        )
        test_data =  torch.concat((test.x[test.pred_edges[:, 0]], test.x[test.pred_edges[:, 1]]), dim=1)
        out = model.predict(test_data)
        result = shared.evaluate(triple, out)
        results.append(result)

    config.output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(config.output_file, 'w') as file:
        json.dump(results, file, indent=2)


if __name__ == "__main__":
    main(Config().parse_args())
