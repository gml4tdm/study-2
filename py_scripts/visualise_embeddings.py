import json
import os
import pathlib

import matplotlib.pyplot as pyplot
import numpy
import tap
import torch
import umap

import shared


class Node:
    def __init__(self):
        self._children = {}
        self.value = None

    def get_child(self, path: list[str]):
        if not path:
            return self
        key, *remainder = path
        assert key
        if key not in self._children:
            self._children[key] = Node()
        node = self._children[key]
        return node.get_child(remainder)

    def apply(self, func):
        children = {
            k: v.apply(func)
            for k, v in self._children.items()
        }
        if self.value is not None:
            value = func(self.value, *(v.value for v in children.values()))
        else:
            value = None
        node = Node()
        node._children = children
        node.value = value
        return node

    def as_dict(self):
        result = {}
        for k1, v1 in self._children.items():
            for k2, v2 in v1.as_dict().items():
                if not k2:
                    result[k1] = v2
                else:
                    result[f'{k1}.{k2}'] = v2
        if self.value is not None:
            result[''] = self.value
        return result



@torch.no_grad()
def get_class_embeddings(source_root: pathlib.Path,
                         embedding_root: pathlib.Path):
    mapping = shared.scan_source_directory(source_root)
    label_mapping = {}
    embeddings = []
    labels = []
    packages = []
    for package, files in mapping.items():
        label = label_mapping.setdefault(package, len(label_mapping))
        for file in files:
            embedding_path = embedding_root / file.with_suffix('.java.bin')
            tensor = torch.load(embedding_path)
            embeddings.append(tensor.cpu().detach().numpy())
            labels.append(label)
            packages.append(package)
    return label_mapping, embeddings, labels


@torch.no_grad()
def get_package_embeddings(source_root: pathlib.Path,
                           embedding_root: pathlib.Path):
    mapping = shared.scan_source_directory(source_root)
    # Convert to a hierarchy
    tree = Node()
    for package, files in mapping.items():
        node = tree.get_child(package.split('.'))
        node.value = [
            torch.load(embedding_root / path.with_suffix('.java.bin')) for path in files
        ]
    # Bottom-up generation of features
    embedding_tree = tree.apply(
        lambda classes, *child_packages: torch.mean(
            torch.stack(classes + list(child_packages)),
            dim=0
        )
    )
    # Get package to embedding mapping
    package_mapping = embedding_tree.as_dict()
    assert '' not in package_mapping
    # Convert to appropriate output format
    label_mapping = {}
    embeddings = []
    labels = []
    for package, embedding in package_mapping.items():
        label_mapping[package] = len(label_mapping)
        embeddings.append(embedding.cpu().detach().numpy())
        labels.append(len(labels))
    return label_mapping, embeddings, labels


def get_as_predictor_embeddings(filename: pathlib.Path):
    with open(filename) as file:
        data = json.load(file)
    order = None
    embeddings = []
    labels = []
    label_mapping = {'not-connected': False, 'connected': True}
    for edge in data:
        labels.append(edge['present_in_graph'])
        if order is None:
            order = list(edge['features'])
        embeddings.append(
            numpy.array([_get(edge['features'], f, edge) for f in order])
        )
    return label_mapping, embeddings, labels


def _get(mapping, key, edge):
    try:
        return mapping[key]
    except KeyError:
        print(f'WARNING: key {key} not found for edge {edge["from"]} -> {edge["to"]}')
        return 0


class Config(tap.Tap):
    source_root: pathlib.Path
    embedding_root: pathlib.Path
    output_path: pathlib.Path
    metric: str = 'euclidean'
    level: str = 'class'
    as_predictor: bool = False

    def configure(self):
        self.add_argument('-s', '--source_root')
        self.add_argument('-e', '--embedding_root')
        self.add_argument('-o', '--output_path')
        self.add_argument('-m', '--metric')
        self.add_argument('-l', '--level')
        self.add_argument('-a', '--as_predictor', action='store_true')


def main(config: Config):
    if config.as_predictor:
        print('WARNING: --level is ignored when running in ASPredictor compatible mode.')
        print('WARNING: --embedding_root is ignored when running in ASPredictor compatible mode.')
        label_mapping, embeddings, labels = get_as_predictor_embeddings(
            config.source_root
        )
    elif config.level == 'class':
        label_mapping, embeddings, labels = get_class_embeddings(
            config.source_root, config.embedding_root
        )
    elif config.level == 'package':
        label_mapping, embeddings, labels = get_package_embeddings(
            config.source_root, config.embedding_root
        )
    else:
        raise NotImplementedError(f'Unknown level {config.level}')

    mapper = umap.UMAP(
        n_neighbors=10,
        min_dist=0.1,
        n_components=2,
        metric=config.metric,
        random_state=42,
    )

    transformed = mapper.fit_transform(numpy.vstack(embeddings))

    fig, ax = pyplot.subplots(figsize=(16, 9))
    scatter = ax.scatter(transformed[:, 0], transformed[:, 1], c=labels)
    leg = ax.legend(
        *scatter.legend_elements(),
        bbox_to_anchor=(1.04, 1),
        borderaxespad=0
    )
    rev_mapping = {v: k for k, v in label_mapping.items()}
    for i in range(len(leg.get_texts())):
        leg.get_texts()[i].set_text(
            #int(leg.get_texts()[i].get_text())
            rev_mapping[i]
        )

    os.makedirs(config.output_path.parent, exist_ok=True)
    fig.savefig(config.output_path, bbox_inches="tight")


if __name__ == '__main__':
    main(Config().parse_args())
