import ast
import json
import os

from matplotlib import pyplot as plt


def get_metrics_for_project(directory: str):
    accuracy = []
    precision = []
    recall = []
    f1_score = []
    keys = []
    for filename in sorted(os.listdir(directory), key=lambda f: get_versions_from_filename(f)[0]):
        path = os.path.join(directory, filename)
        with open(path) as file:
            data = json.load(file)
        versions = get_versions_from_filename(filename)
        key = '/'.join('.'.join(map(str, v)) for v in versions)
        keys.append(key)
        accuracy.append(data['accuracy'])
        precision.append(data['precision'])
        recall.append(data['recall'])
        f1_score.append(data['f1_score'])
    return accuracy, precision, recall, f1_score, keys


def get_versions_from_filename(filename):
    return [
        ast.literal_eval(x)
        for x in filename.removesuffix('.json').split('__')
    ]


def make_bar_plots(project, accuracy, precision, recall, f1_score, keys):
    fig, axes = plt.subplots(nrows=2, ncols=2)
    fig.set_size_inches(19.20, 10.80)
    (ax1, ax2), (ax3, ax4) = axes

    ax1.barh(range(len(keys)), accuracy)
    ax1.set_yticks(range(len(keys)))
    ax1.set_yticklabels(keys)
    ax1.set_title('Accuracy')
    ax1.set_xlim([0, 1])

    ax2.barh(range(len(keys)), f1_score)
    ax2.set_yticks(range(len(keys)))
    ax2.set_yticklabels(keys)
    ax2.set_title('F1 Score')
    ax2.set_xlim([0, 1])

    ax3.barh(range(len(keys)), precision)
    ax3.set_yticks(range(len(keys)))
    ax3.set_yticklabels(keys)
    ax3.set_title('Precision')
    ax3.set_xlim([0, 1])

    ax4.barh(range(len(keys)), recall)
    ax4.set_yticks(range(len(keys)))
    ax4.set_yticklabels(keys)
    ax4.set_title('Recall')
    ax4.set_xlim([0, 1])

    fig.tight_layout()
    os.makedirs('./figures', exist_ok=True)
    fig.savefig(f'./figures/{project}.png')


def main():
    for directory in os.listdir('./results'):
        make_bar_plots(
            directory,
            *get_metrics_for_project(os.path.join('./results', directory))
        )


if __name__ == '__main__':
    main()
