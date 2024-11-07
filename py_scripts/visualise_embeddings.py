import os
import pathlib
import re

import matplotlib.pyplot as pyplot
import numpy
import tap
import torch
import umap


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


class Config(tap.Tap):
    source_root: pathlib.Path
    embedding_root: pathlib.Path

    def configure(self):
        self.add_argument('-s', '--source_root')
        self.add_argument('-e', '--embedding_root')


def main(config: Config):
    mapping = scan_source_directory(config.source_root)
    label_mapping = {}
    embeddings = []
    labels = []
    packages = []
    file_order = []
    for package, files in mapping.items():
        label = label_mapping.setdefault(package, len(label_mapping))
        for file in files:
            file_order.append(file)
            embedding_path = config.embedding_root / file.with_suffix('.java.bin')
            tensor = torch.load(embedding_path)
            embeddings.append(tensor.cpu().detach().numpy())
            labels.append(label)
            packages.append(package)

    mapper = umap.UMAP(
        n_neighbors=10,
        min_dist=0.1,
        n_components=2,
        metric='euclidean',
        random_state=42,
    )

    transformed = mapper.fit_transform(numpy.vstack(embeddings))

    fig, ax = pyplot.subplots(figsize=(16, 9))
    scatter = ax.scatter(transformed[:, 0], transformed[:, 1], c=labels)
    leg = ax.legend(
        *scatter.legend_elements(),
        loc='upper right'
    )
    rev_mapping = {v: k for k, v in label_mapping.items()}
    for i in range(len(leg.get_texts())):
        leg.get_texts()[i].set_text(
            #int(leg.get_texts()[i].get_text())
            rev_mapping[i]
        )
    for (x, y), label in zip(transformed, file_order, strict=True):
        print(x, y, label)
    pyplot.show()


if __name__ == '__main__':
    main(Config().parse_args())
