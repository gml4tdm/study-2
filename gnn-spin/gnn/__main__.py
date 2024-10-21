################################################################################
################################################################################
# Imports
################################################################################

import logging
import os
import pathlib
import sys

import tap
import torch

################################################################################
################################################################################
# Define Parameters
################################################################################


class Config(tap.Tap):
    output_directory: pathlib.Path
    model_config: pathlib.Path
    data_directory: pathlib.Path


################################################################################
################################################################################
# Data Preparation
################################################################################


def get_version_triples(path: pathlib.Path):
    versions = []
    for directory in os.listdir(path):
        version = tuple(
            int(x) if x.isdigit() else x
            for x in directory.split('.')
        )
        versions.append(version)
    versions.sort()
    result = []
    for i in range(len(versions) - 3 + 1):
        triple = versions[i:i + 3]
        result.append(triple)
    return result


################################################################################
################################################################################
# Program Entrypoint
################################################################################


def setup_logging(log_file_path: pathlib.Path) -> logging.Logger:
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

    stream_handler = logging.StreamHandler(sys.stdout)
    stream_handler.setFormatter(formatter)
    stream_handler.setLevel(logging.INFO)
    logger.addHandler(stream_handler)

    return logger


def main(args: Config):
    logger = setup_logging(args.output_directory / 'log.txt')


if __name__ == '__main__':
    main(Config().parse_args())
