################################################################################
################################################################################
# Imports
################################################################################

import argparse
import contextlib
import dataclasses
import json
import logging
import os
import pathlib
import sys
from typing import Iterator

import alive_progress
import torch
import transformers

CPU = 'cpu'
GPU = 'cuda:0'
DEVICE = GPU if torch.cuda.is_available() else CPU

################################################################################
################################################################################
# Auxiliary Types
################################################################################


@dataclasses.dataclass(frozen=True)
class WorkItem:
    source: pathlib.Path
    target: pathlib.Path
    file_size: int


################################################################################
################################################################################
# Main Implementation Classes
################################################################################


class PretrainedHuggingFaceModelWithTokenizer:

    def __init__(self,
                 name: str, *,
                 tokenizer: str | None = None,
                 load_on_init: bool = True):
        self._tokenizer_name = tokenizer if tokenizer is not None else name
        self._model_name = name
        self._tokenizer = None
        self._model = None
        if load_on_init:
            self._load_model()

    def _load_model(self):
        self._tokenizer = transformers.AutoTokenizer.from_pretrained(
            self._tokenizer_name,
            trust_remote_code=True
        )
        self._model = transformers.AutoModel.from_pretrained(
            self._model_name,
            trust_remote_code=True
        ).to(DEVICE)

    def _model_is_loaded(self) -> bool:
        return self._model is not None

    def tokenise(self, texts: list[str]):
        return self._tokenizer(
            texts,
            return_tensors='pt',
            padding=True,
            truncation=True
        ).to(DEVICE)

    def predict(self, texts: list[str]):
        if not self._model_is_loaded():
            self._load_model()
        inputs = self.tokenise(texts)
        outputs = self._model(**inputs)
        return outputs


################################################################################
################################################################################
# Main Implementation Functions
################################################################################


def scan_input_tree(input_directory: pathlib.Path,
                    output_mirror_dir: pathlib.Path,
                    logger: logging.Logger) -> list[WorkItem]:
    items = []
    for source_base_path, dirs, files in os.walk(input_directory):
        source_base_path = pathlib.Path(source_base_path)
        relative_path = source_base_path.relative_to(input_directory)
        target_base_path = output_mirror_dir / relative_path
        target_base_path.mkdir(exist_ok=True)
        for filename in files:
            source = source_base_path / filename
            target = target_base_path / f'{filename}.bin'
            size = source.stat().st_size
            logger.debug(
                f'Registering filename pair: %s -> %s (size: %s bytes)',
                source, target, size
            )
            item = WorkItem(source, target, size)
            items.append(item)
    return items


def filter_items_on_status(items: list[WorkItem],
                           done: list[str]) -> list[WorkItem]:
    done = set(done)
    return [item for item in items if str(item.source) not in done]


def batch_work_items(items: list[WorkItem],
                     limit: int) -> Iterator[list[WorkItem]]:
    stack = items.copy()
    current_batch = []
    current_size = 0
    while stack:
        item = stack.pop()
        if current_size + 1 > limit:
            yield current_batch
            current_size = 1
            current_batch = [item]
        else:
            current_size += 1
            current_batch.append(item)
    if current_batch:
        yield current_batch


def load_text_data(items: list[WorkItem]) -> list[str]:
    result = []
    for item in items:
        with open(item.source, 'r', errors='ignore') as file:
            result.append(file.read())
    return result


def store_embedding_data(items: list[WorkItem], embeddings):
    moved = embeddings.to(CPU)
    print(f'Number of work items: {len(items)}')
    print(f'Tensor shape: {moved.shape}')
    for item, embedding in zip(items, torch.unbind(moved), strict=True):
        print(f'Storing tensor of shape {embedding.shape} ({item.target})')
        torch.save(embedding.clone(), item.target)


def update_status_file(path: pathlib.Path, items: list[WorkItem]):
    with open(path, 'r') as file:
        status = json.load(file)
    status.extend([str(item.source) for item in items])
    with open(path, 'w') as file:
        json.dump(status, file)


################################################################################
################################################################################
# Program Entry Point
################################################################################



def setup_logging(log_file_path: pathlib.Path,
                  interactive: bool) -> logging.Logger:
    logger = logging.getLogger(__name__)
    logger.setLevel(logging.DEBUG)
    formatter = logging.Formatter(
        '[{name}][{asctime}][{levelname:8}]: {message}',
        style='{'
    )
    file_handler = logging.FileHandler(log_file_path, mode='w')
    file_handler.setFormatter(formatter)
    file_handler.setLevel(logging.DEBUG)
    logger.addHandler(file_handler)
    if interactive:
        stream_handler = logging.StreamHandler(sys.stdout)
        stream_handler.setFormatter(formatter)
        stream_handler.setLevel(logging.INFO)
        logger.addHandler(stream_handler)
    return logger


def main(input_directory: pathlib.Path,
         output_directory: pathlib.Path,
         batch_limit: int,
         interactive: bool = False):
    output_directory.mkdir(exist_ok=True)
    status_file_path = output_directory / 'status.json'
    output_mirror_dir = output_directory / 'output'
    log_file_path = output_directory / 'log.txt'

    logger = setup_logging(log_file_path, interactive)
    work_items = scan_input_tree(input_directory, output_mirror_dir, logger)
    logger.info(f'Found %s files.', len(work_items))

    if not status_file_path.exists():
        with open(status_file_path, 'w') as file:
            json.dump([], file)
    with open(status_file_path) as file:
        work_items = filter_items_on_status(work_items, json.load(file))
    logger.info(f'%s files remaining after status check.', len(work_items))

    batches = list(batch_work_items(work_items, batch_limit))
    logger.info('Predicting in %s batches.', len(batches))

    logger.info('Loading model...')
    model = PretrainedHuggingFaceModelWithTokenizer(
        'Salesforce/codet5p-110m-embedding', load_on_init=True
    )

    logger.info('Performing batch prediction...')
    if interactive:
        context = alive_progress.alive_bar(len(batches))
    else:
        @contextlib.contextmanager
        def _ctx():
            yield lambda: None
        context = _ctx()
    with context as bar:
        for i, batch in enumerate(batches):
            logger.info('Processing batch %s (%s items)', i + 1, len(batch))
            inputs = load_text_data(batch)
            with torch.no_grad():
                embeddings = model.predict(inputs)
                store_embedding_data(batch, embeddings)
            update_status_file(status_file_path, batch)
            bar()



if __name__ == '__main__':
    parser = argparse.ArgumentParser(__name__)
    parser.add_argument('--input-directory', type=str, required=True)
    parser.add_argument('--output-directory', type=str, required=True)
    parser.add_argument('--batch-limit', type=int, default=10_000)
    parser.add_argument('--interactive', action='store_true', default=False)
    args = parser.parse_args()
    main(
        input_directory=pathlib.Path(args.input_directory),
        output_directory=pathlib.Path(args.output_directory),
        batch_limit=args.batch_limit,
        interactive=args.interactive
    )
