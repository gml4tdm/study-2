import contextlib
import pathlib

import matplotlib.pyplot as pyplot
from transformers.models.flava.modeling_flava import FlavaSelfOutput


@contextlib.contextmanager
def simple_figure(*, size: tuple[float, float] | None = None, filename = None):
    if size is None:
        # Default to a 16:9 aspect ratio
        size = (16, 9)
    fig, ax = pyplot.subplots(figsize=size)
    yield fig, ax
    fig.tight_layout()
    if filename is None:
        pyplot.show()
    else:
        fig.savefig(filename)


def bar_variable(series_by_name: dict[str, list[float]],
                 labels: list[str],
                 title: str,
                 output_file: pathlib.Path,
                 show_legend: bool = True,
                 rotate_text: bool = False):
    with simple_figure(filename=output_file) as (fig, ax):
        width = 0.8 / len(series_by_name)
        n = None
        for j, (name, series) in enumerate(series_by_name.items()):
            x = [
                i - (len(series_by_name) - 1)/2*width + j*width
                for i in range(len(series))
            ]
            if n is not None and len(x) != n:
                raise ValueError('Value arrays not of equal length')
            n = len(x)
            ax.bar(x, series, label=name, width=width)
            for x, v in zip(x, series):
                ax.text(x, v, f'{v:.2f}',
                        horizontalalignment='center',
                        verticalalignment='bottom',
                        rotation=0 if not rotate_text else 90)
        ax.set_xticks(range(n))
        ax.set_xticklabels(labels)
        ax.set_title(title)
        if show_legend:
            ax.legend(loc='upper left')