import itertools
import pathlib

import matplotlib.pyplot as pyplot
import tap

import shared


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    output_directory: pathlib.Path

    def configure(self):
        self.add_argument('-i', '--input_files')
        self.add_argument('-o', '--output_directory')


def convert_edges(g: shared.Graph):
    return {
        (g.nodes[i].name, g.nodes[j].name): flag
        for (i, j), flag in zip(g.edge_labels.edges, g.edge_labels.labels)
    }


def modified_labels(lhs: shared.Graph, rhs: shared.Graph):
    lhs_edges = convert_edges(lhs)
    rhs_edges = convert_edges(rhs)
    common_edges = set(lhs_edges.keys()) & set(rhs_edges.keys())
    return sum(lhs_edges[k] != rhs_edges[k] for k in common_edges) / len(lhs_edges)


def added_labels(lhs: shared.Graph, rhs: shared.Graph):
    lhs_edges = convert_edges(lhs)
    rhs_edges = convert_edges(rhs)
    added_edges = set(rhs_edges.keys()) - set(lhs_edges.keys())
    return len(added_edges) / len(lhs_edges)


def deleted_labels(lhs: shared.Graph, rhs: shared.Graph):
    lhs_edges = convert_edges(lhs)
    rhs_edges = convert_edges(rhs)
    deleted_edges = set(lhs_edges.keys()) - set(rhs_edges.keys())
    return len(deleted_edges) / len(lhs_edges)


def unmodified_labels(lhs: shared.Graph, rhs: shared.Graph):
    lhs_edges = convert_edges(lhs)
    rhs_edges = convert_edges(rhs)
    common_edges = set(lhs_edges.keys()) & set(rhs_edges.keys())
    return sum(lhs_edges[k] == rhs_edges[k] for k in common_edges) / len(lhs_edges)


def main(config: Config):
    print('Loading files...')
    loaded = [
        shared.VersionTriple.load_and_check(filename)
        for filename in config.input_files
    ]
    print('Computing statistics...')
    dataset_sizes_by_project = {}
    for triple in loaded:
        dataset_sizes_by_project.setdefault(triple.project, []).append(
            len(triple.training_graph.edge_labels.labels)
        )
    label_similarity_by_project = {}
    label_additions_by_project = {}
    label_deletions_by_project = {}
    label_modifications_by_project = {}
    for triple in loaded:
        print(f'Processing {triple.project} ({triple.version_1}, {triple.version_2}, {triple.version_3})')
        label_similarity_by_project.setdefault(triple.project, []).append(
            unmodified_labels(triple.training_graph, triple.test_graph)
        )
        label_additions_by_project.setdefault(triple.project, []).append(
            added_labels(triple.training_graph, triple.test_graph)
        )
        label_deletions_by_project.setdefault(triple.project, []).append(
            deleted_labels(triple.training_graph, triple.test_graph)
        )
        label_modifications_by_project.setdefault(triple.project, []).append(
            modified_labels(triple.training_graph, triple.test_graph)
        )
    # We generate a graph for every project, plus an overview graph.
    for project in dataset_sizes_by_project:
        fig = pyplot.figure(figsize=(16, 9))
        grid = pyplot.GridSpec(2, 3, figure=fig)
        ax1 = fig.add_subplot(grid[:, 0])
        ax2 =  fig.add_subplot(grid[0, 1])
        ax3 =  fig.add_subplot(grid[0, 2])
        ax4 =  fig.add_subplot(grid[1, 1])
        ax5 =  fig.add_subplot(grid[1, 2])
        sets = [
            dataset_sizes_by_project,
            label_similarity_by_project,
            label_modifications_by_project,
            label_additions_by_project,
            label_deletions_by_project,
        ]
        axes = [ax1, ax2, ax3, ax4, ax5]
        titles = [
            "Dataset Size",
            "Label Similarity",
            "Label Modifications",
            "Label Additions",
            "Label Deletions",
        ]
        for ax, mapping, title in zip(axes, sets, titles):
            ax.violinplot(
                mapping[project],
                showmeans=True,
                showextrema=True,
                showmedians=True,
            )
            ax.set_title(title)
        fig.savefig(config.output_directory / f'{project}_global_triple_statistics.png')
    # Cumulative statistics
    fig = pyplot.figure(figsize=(16, 9))
    grid = pyplot.GridSpec(2, 3, figure=fig)
    ax1 = fig.add_subplot(grid[:, 0])
    ax2 = fig.add_subplot(grid[0, 1])
    ax3 = fig.add_subplot(grid[0, 2])
    ax4 = fig.add_subplot(grid[1, 1])
    ax5 = fig.add_subplot(grid[1, 2])
    sets = [
        dataset_sizes_by_project,
        label_similarity_by_project,
        label_modifications_by_project,
        label_additions_by_project,
        label_deletions_by_project,
    ]
    axes = [ax1, ax2, ax3, ax4, ax5]
    titles = [
        "Dataset Size",
        "Label Similarity",
        "Label Modifications",
        "Label Additions",
        "Label Deletions",
    ]
    for ax, mapping, title in zip(axes, sets, titles):
        ax.violinplot(
            list(itertools.chain(*mapping.values())),
            showmeans=True,
            showextrema=True,
            showmedians=True,
        )
        ax.set_title(title)
    fig.savefig(config.output_directory / f'global_triple_statistics.png')


if __name__ == "__main__":
    main(Config().parse_args())
