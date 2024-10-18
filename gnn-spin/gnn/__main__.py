################################################################################
################################################################################
# Imports
################################################################################

import logging
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
