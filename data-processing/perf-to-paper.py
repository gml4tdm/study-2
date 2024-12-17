import collections
import json
import os
import pathlib
import statistics
import sys

import pandas
import tap

import matplotlib.pyplot as pyplot
import seaborn
import ptitprince


class Config(tap.Tap):
    files: list[pathlib.Path]
    name_mapping: str
    metrics: list[str]


def _build_df(mapping: list[tuple[str, str]]):
    values = []
    titles = []
    for key, value in mapping:
        values.extend(value)
        titles.extend([key] * len(value))
    return pandas.DataFrame({'title': titles, 'value': values})


def main(conf: Config):
    name_mapping = {
        k: v
        for k, v in (
            part.split("=")
            for part in conf.name_mapping.split(",")
        )
    }
    averages = collections.defaultdict(dict)
    distributions = collections.defaultdict(dict)

    for filename in conf.files:
        with open(filename) as file:
            data = json.load(file)
        for metric in conf.metrics:
            dist = [entry[metric] for entry in data['scores']]
            dist = [x if x is not None else 0.0 for x in dist]
            averages[str(filename.stem)][metric] = (
                (
                    sum(dist) / len(dist),
                    statistics.stdev(dist)
                )
            )
            distributions[str(filename.stem)][metric] = dist

    # Generate LaTeX table output
    for filename, metrics in averages.items():
        parts = [name_mapping[filename]]
        for metric in conf.metrics:
            parts.append(f"{metrics[metric][0]:.2f} \\pm {metrics[metric][1]:.3f}")
        print(" & ".join(parts) + '\\\\')

    # Generate Figure
    fig = pyplot.figure(figsize=(6, 6))
    grid = pyplot.GridSpec(1, 1, figure=fig)
    ax1 = fig.add_subplot(grid[0, 0])
    #ax2 = fig.add_subplot(grid[1, 0])


    df_data = {
       'source': [],
       'metric': [],
        'value': []
    }
    for filename, metrics in distributions.items():
        for metric, values in metrics.items():
            for value in values:
                df_data['source'].append(name_mapping[filename])
                df_data['metric'].append(metric)
                df_data['value'].append(value)
    df1 = pandas.DataFrame(df_data)
    print(df1)

    for ax, df in zip([ax1], [df1]):
        # Dirty hack to disable the boxplots
        original = seaborn.boxplot
        seaborn.boxplot = lambda *args, **kwargs: None

        vio_original = ptitprince.half_violinplot
        ptitprince.half_violinplot = lambda *args, **kwargs: vio_original(*args, **kwargs, inner='quart')

        cloud = ptitprince.RainCloud(
            x='metric',
            y='value',
            hue='source',
            data=df,
            ax=ax,
            width_viol=0.5,
            width_box=0.1,
            alpha=0.65,
            scale='area'
        )
        seaborn.boxplot = original
        ptitprince.half_violinplot = vio_original

        handles, labels = ax.get_legend_handles_labels()

        # Fix the legend, since it is messed up because of our boxplot hack
        n_plots = 2
        _ = ax.legend(handles[0:len(labels) // n_plots], labels[0:len(labels) // n_plots],
                       bbox_to_anchor=(1.05, 1), loc=2, borderaxespad=0.,
                       title='Source')  # , title_fontsize = 25)

    fig.tight_layout()
    fig.savefig('paper.png')


if __name__ == "__main__":
    main(Config().parse_args())
