################################################################################
################################################################################
# Imports
################################################################################

from __future__ import annotations

import operator
import pathlib
import typing

import pydantic
import tap

from shared_plotting import simple_figure, bar_variable

################################################################################
################################################################################
# Data Schema
################################################################################

class ProjectEvolutionStatistics(pydantic.BaseModel):
    project: str
    versions: list[str]
    graphs_per_version: list[GraphStatistics]
    vertices_per_version: list[VertexStatistics]
    edges_per_version: list[EdgeStatistics]
    vertex_edits_per_version: list[VertexEditStatistics]
    edge_edits_per_version: list[EdgeEditStatistics]

    @classmethod
    def load_and_check(cls, filename) -> typing.Self:
        with open(filename, 'r') as file:
            self = pydantic.TypeAdapter(cls).validate_json(file.read())
        return self


class GraphStatistics(pydantic.BaseModel):
    diameter: int
    hops: Statistics


class VertexStatistics(pydantic.BaseModel):
    total: int

    in_degree: Statistics
    in_degree_no_duplicates: Statistics
    in_degree_no_self: Statistics
    in_degree_no_self_no_duplicates: Statistics

    in_degree_by_type: dict[str, Statistics]
    in_degree_by_type_no_duplicates: dict[str, Statistics]
    in_degree_by_type_no_self: dict[str, Statistics]
    in_degree_by_type_no_self_no_duplicates: dict[str, Statistics]

    out_degree: Statistics
    out_degree_no_duplicates: Statistics
    out_degree_no_self: Statistics
    out_degree_no_self_no_duplicates: Statistics

    out_degree_by_type: dict[str, Statistics]
    out_degree_by_type_no_duplicates: dict[str, Statistics]
    out_degree_by_type_no_self: dict[str, Statistics]
    out_degree_by_type_no_self_no_duplicates: dict[str, Statistics]


class EdgeStatistics(pydantic.BaseModel):
    total: int

    total_no_duplicates: int
    total_no_self: int
    total_no_self_no_duplicates: int

    total_by_type: dict[str, int]
    total_by_type_no_duplicates: dict[str, int]
    total_by_type_no_self: dict[str, int]
    total_by_type_no_self_no_duplicates: dict[str, int]


class VertexEditStatistics(pydantic.BaseModel):
    added: int
    deleted: int


class EdgeEditStatistics(pydantic.BaseModel):
    added: int
    added_no_duplicates: int
    added_no_self: int
    added_no_self_no_duplicates: int

    added_by_type: dict[str, int]
    added_by_type_no_duplicates: dict[str, int]
    added_by_type_no_self: dict[str, int]
    added_by_type_no_self_no_duplicates: dict[str, int]

    deleted: int
    deleted_no_duplicates: int
    deleted_no_self: int
    deleted_no_self_no_duplicates: int

    deleted_by_type: dict[str, int]
    deleted_by_type_no_duplicates: dict[str, int]
    deleted_by_type_no_self: dict[str, int]
    deleted_by_type_no_self_no_duplicates: dict[str, int]


class Statistics(pydantic.BaseModel):
    mean: float
    median: float
    std_dev: float


################################################################################
################################################################################
# Plotting
################################################################################


def plot_time_series_statistics(series: list[Statistics],
                                labels: list[str],
                                title: str,
                                output_file: pathlib.Path):
    with simple_figure() as (fig, ax):
        ax.errorbar(
            range(len(series)),
            [s.mean for s in series],
            yerr=[s.std_dev for s in series],
            label='Mean (std dev)',
        )
        ax.plot(
            range(len(series)),
            [s.median for s in series],
            label='Median',
        )
        ax.set_xticks(range(len(series)))
        ax.set_xticklabels(labels)
        ax.set_title(title)
        ax.legend(loc='upper left')


def plot_time_series_int(series: list[int],
                         labels: list[str],
                         title: str,
                         output_file: pathlib.Path):
    with simple_figure() as (fig, ax):
        ax.plot(
            range(len(series)),
            series,
            label='Values',
        )
        ax.set_xticks(range(len(series)))
        ax.set_xticklabels(labels)
        ax.set_title(title)
        ax.legend(loc='upper left')


def bar_time_series_statistics(series: list[Statistics],
                               labels: list[str],
                               title: str,
                               output_file: pathlib.Path):
   bar_variable(
       {
           'Mean (+- Std)': [v.mean for v in series],
           'Median': [v.median for v in series]
       },
       labels,
       title,
       output_file
   )


def bar_time_series_int(series: list[int],
                        labels: list[str],
                        title: str,
                        output_file: pathlib.Path):
    bar_variable(
        {'_': series}, labels, title, output_file, show_legend=False
    )


def bar_evolution(stats: ProjectEvolutionStatistics,
                  node_or_edge: str,
                  which: str | None = None,
                  output_directory: pathlib.Path = None):
    if node_or_edge == 'vertices':
        main_getter = operator.attrgetter('vertices_per_version')
        edit_getter = operator.attrgetter('vertex_edits_per_version')
    else:
        main_getter = operator.attrgetter('edges_per_version')
        edit_getter = operator.attrgetter('edge_edits_per_version')
    total_getter = operator.attrgetter(f'total_{which}' if which else 'total')
    add_getter = operator.attrgetter(f'added_{which}' if which else 'added')
    del_getter = operator.attrgetter(f'deleted_{which}' if which else 'deleted')
    total = [total_getter(v) for v in main_getter(stats)]
    additions = [add_getter(v) for v in edit_getter(stats)]
    deletions = [del_getter(v) for v in edit_getter(stats)]
    additions.insert(0, 0)
    deletions.insert(0, 0)
    name = 'Vertices' if node_or_edge == 'vertices' else 'Edges'
    suffix = f'__{which}' if which else ''
    filename = f'{stats.project}__{name.lower()}_by_version{suffix}.png'
    bar_variable(
        {
            'Total': total,
            'Added': additions,
            'Deleted': deletions
        },
        stats.versions,
        f'{name} per Version ({stats.project})',
       output_directory / filename
    )


################################################################################
################################################################################
# Main
################################################################################


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    output_directory: pathlib.Path

    def configure(self):
        self.add_argument('-i', '--input_files')
        self.add_argument('-o', '--output_directory')


def main(config: Config):
    config.output_directory.mkdir(parents=True, exist_ok=True)
    for filename in config.input_files:
        print(f'Plotting {filename}')
        stats = ProjectEvolutionStatistics.load_and_check(filename)
        bar_evolution(stats, 'edges', 'no_self_no_duplicates', config.output_directory)
        bar_evolution(stats, 'edges', 'no_duplicates', config.output_directory)
        bar_evolution(stats, 'vertices', None, config.output_directory)


if __name__ == '__main__':
    main(Config().parse_args())
