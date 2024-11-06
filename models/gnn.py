################################################################################
################################################################################
# Imports
################################################################################

import os
import pathlib
import re

import torch
import torch_geometric

import tap

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
    print(len(graph.nodes))
    feat = build_node_features(graph, source_directory, embedding_directory)
    print(feat.shape)
    return torch_geometric.data.Data(
        x=feat,
        edge_index=torch.tensor([
            [edge.from_ for edge in graph.edges],
            [edge.to for edge in graph.edges]
        ]),
        pred_edges=torch.tensor(graph.edge_labels.edges),
        y=torch.tensor(graph.edge_labels.labels, dtype=torch.float)
    )


def build_node_features(graph: shared.Graph,
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
        hierarchy: shared.NodeHierarchy,
        feature_map: dict[int, torch.Tensor],
        file_mapping: dict[str, list[pathlib.Path]],
        graph: shared.Graph,
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
    pattern = re.compile(r'^package\s+(?P<package>[a-zA-Z0-9_.]+);')
    with open(path, encoding='utf-8', errors='ignore') as file:
        for line in file:
            if (m := pattern.match(line)) is not None:
                return m.group('package')
    raise ValueError(f'Could not find package in {path}')


def load_embedding(path: pathlib.Path, embedding_directory: pathlib.Path):
    path = embedding_directory / path.with_suffix(f'{path.suffix}.bin')
    print(f'Loading {path}')
    return torch.load(path)


################################################################################
################################################################################
# Model
################################################################################


class Model(torch.nn.Module):

    def __init__(self, embedding_in: int):
        super().__init__()
        self.conv1 = torch_geometric.nn.GCNConv(embedding_in, 16)
        self.conv2 = torch_geometric.nn.GCNConv(16, 8)
        self.linear = torch.nn.Linear(8, 1)

    def forward(self, x):
        z = x
        x = self.conv1(x.x, z.edge_index)
        x = self.conv2(x, z.edge_index)
        x = torch.flatten(x, 1)
        x = self.linear(x)
        x = torch.sigmoid(x)
        x = torch.flatten(x, 1)

        # Link prediction
        matrix = x[z.pred_edges]
        pred = torch.mul(matrix[:, 0], matrix[:, 1])
        return pred.transpose(0, 1).flatten(0)


################################################################################
################################################################################
# Program Entrypoint
################################################################################


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    source_directory: pathlib.Path
    embedding_directory: pathlib.Path


def main(config: Config):
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

        # Training
        device = 'cuda' if torch.cuda.is_available() else 'cpu'
        model = Model(256)
        model.to(device)
        train.to(device)
        optimizer = torch.optim.Adam(model.parameters(), lr=0.001)
        loss_fn = torch.nn.BCELoss()
        for epoch in range(1000):
            print(epoch)
            out = model(train)
            print(out)
            print(train.y)
            loss = loss_fn(out, train.y)
            print(loss)
            loss.backward()
            optimizer.step()
            optimizer.zero_grad()

        # Evaluation
        del train
        test = get_pytorch_dataset(
            triple.test_graph,
            config.source_directory / triple.project / triple.version_2 / SOURCE_DIRECTORY[key_2],
            config.embedding_directory / triple.project / triple.version_2 / SOURCE_DIRECTORY[key_2],
        )
        with torch.no_grad():
            out = model(test).tolist()
            exp = test.y.tolist()
            tp = fp = tn = fn = 0
            for pred, true in zip(out, exp):
                if pred > 0.5 and true > 0.5:
                    tp += 1
                elif pred > 0.5 and true <= 0.5:
                    fp += 1
                elif pred <= 0.5 and true > 0.5:
                    fn += 1
                else:
                    tn += 1
            print(f'True Positive: {tp}')
            print(f'False Positive: {fp}')
            print(f'True Negative: {tn}')
            print(f'False Negative: {fn}')


if __name__ == "__main__":
    main(Config().parse_args())
