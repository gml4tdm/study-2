import pathlib

import tap

import shared
import shared_plotting


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    output_directory: pathlib.Path

    def configure(self):
        self.add_argument('-i', '--input_files')
        self.add_argument('-o', '--output_directory')



def stats_for_triple(triple: shared.VersionTriple):
    result = {
        f'train-{k}': v
        for k, v in stats_for_graph(triple.training_graph).items()
    }
    for k, v in stats_for_graph(triple.test_graph).items():
        result[f'test-{k}'] = v
    return result


def stats_for_graph(graph: shared.Graph):
    return {
        'vertices': len(graph.nodes),
        'edges': len(graph.edges),
        #'prediction edges': len(graph.edge_labels.edges),
        'true samples': sum(graph.edge_labels.labels),
        #'false samples': len(graph.edge_labels.labels) - sum(graph.edge_labels.labels),
    }



def main(config: Config):
    aggregated = {}
    labels = []
    project = None
    loaded = [
        shared.VersionTriple.load_and_check(filename)
        for filename in config.input_files
    ]
    loaded.sort(
        key=lambda triple: tuple([
            int(x) if x.isdigit() else x for x in triple.version_1.split('.')
        ])
    )
    for triple in loaded:
        print(f'Processing {triple.project} ({triple.version_1}, {triple.version_2}, {triple.version_3})')
        labels.append('\n'.join([triple.version_1, triple.version_2, triple.version_3]))
        if project is not None and project != triple.project:
            raise ValueError('Projects do not match')
        project = triple.project
        for k, v in stats_for_triple(triple).items():
            aggregated.setdefault(k, []).append(v)

    config.output_directory.mkdir(parents=True, exist_ok=True)

    shared_plotting.bar_variable(
        aggregated,
        labels,
        f'Triple Statistics ({project})',
        config.output_directory / f'{project}_triple_statistics.png',
        rotate_text=True
    )



if __name__ == "__main__":
    main(Config().parse_args())
