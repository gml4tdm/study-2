################################################################################
################################################################################
# Imports
################################################################################

from __future__ import annotations

import functools
import json
import logging
import os
import pathlib
import sys
import traceback
import typing

import pydantic
import pydantic_xml
import tap
import torch

################################################################################
################################################################################
# Define Parameters
################################################################################


class Config(tap.Tap):
    output_directory: pathlib.Path
    model_config: pathlib.Path
    embedding_directory: pathlib.Path
    graph_directory: pathlib.Path
    structure_directory: pathlib.Path
    exclude_projects: list[str]
    project_legacy_mapping: str = ''


################################################################################
################################################################################
# ODEM Schema
################################################################################


class ODEM(pydantic_xml.BaseXmlModel):
    version: str = pydantic_xml.attr()
    header: Header = pydantic_xml.element()
    contexts: list[Context] = pydantic_xml.element(tag='context')


class Header(pydantic_xml.BaseXmlModel):
    created_by: CreatedBy = pydantic_xml.element(tag='created-by')


class CreatedBy(pydantic_xml.BaseXmlModel):
    exporter: Exporter = pydantic_xml.element(tag='exporter')
    provider: Provider = pydantic_xml.element(tag='provider')


class Exporter(pydantic_xml.BaseXmlModel):
    version: str = pydantic_xml.attr()
    name: str


class Provider(pydantic_xml.BaseXmlModel):
    name: str


class Context(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    containers: list[Container] = pydantic_xml.element(tag='container',
                                                       default=[])


class Container(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    classification: typing.Literal[
        'jar', 'osgi-bundle'] = pydantic_xml.attr()
    namespaces: list[Namespace] = pydantic_xml.element(tag='namespace',
                                                       default=[])


class Namespace(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    types: list[Type] = pydantic_xml.element(tag='type', default=[])


class Type(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    classification: typing.Literal[
        'class', 'interface', 'enum',
        'annotation', 'unknown'] = pydantic_xml.attr()
    visibility: typing.Literal[
        'public', 'protected', 'private', 'default'] = pydantic_xml.attr()
    dependencies: Dependencies = pydantic_xml.element()


class Dependencies(pydantic_xml.BaseXmlModel):
    count: int = pydantic_xml.attr()
    depends_on: list[DependsOn] = pydantic_xml.element(tag='depends-on',
                                                       default=[])


class DependsOn(pydantic_xml.BaseXmlModel):
    name: str = pydantic_xml.attr()
    classification: typing.Literal[
        'extends', 'uses', 'implements'] = pydantic_xml.attr()


ODEM.model_rebuild()
Header.model_rebuild()
CreatedBy.model_rebuild()
Exporter.model_rebuild()
Provider.model_rebuild()
Context.model_rebuild()
Container.model_rebuild()
Namespace.model_rebuild()
Type.model_rebuild()
Dependencies.model_rebuild()
DependsOn.model_rebuild()



################################################################################
################################################################################
# Data Preparation
################################################################################


def get_version_triples(path: pathlib.Path):
    versions = []
    for directory in os.listdir(path):
        cleaned = directory.removesuffix('.json').removesuffix('.odem')
        version = tuple(
            int(x) if x.isdigit() else x
            for x in cleaned.split('.')
        )
        versions.append(version)
    versions.sort()
    result = []
    for i in range(len(versions) - 3 + 1):
        triple = versions[i:i + 3]
        result.append(triple)
    return result


def find_files_for_version(project: str, version: str, *,
                           graph_directory: pathlib.Path,
                           structure_directory: pathlib.Path,
                           embeddings_directory: pathlib.Path,
                           legacy_mapping: dict[str, str]):
    structure_file = structure_directory / project / f'{version}.json'
    embedding_directory = embeddings_directory / project / version
    inner_project = legacy_mapping.get(project, project)
    graph_file = graph_directory / project / f'{inner_project}-{version}.odem'
    return structure_file, embedding_directory, graph_file


################################################################################
################################################################################
# JIT Feature Utilities
################################################################################


def build_module_features(structure_file: pathlib.Path,
                          embedding_directory: pathlib.Path):
    with open(structure_file) as file:
        structure = json.load(file)
    raw = _build_module_features_recursively(structure, embedding_directory)
    flattened = _flatten_features(raw)
    root = flattened.pop('')
    flattened['@project'] = root
    return flattened


def _build_module_features_recursively(structure,
                                       embedding_directory: pathlib.Path):
    kind = structure.pop('type')
    if kind == 'Entity':
        return torch.load(embedding_directory / structure['path'])
    else:
        result = {}
        assert kind == 'Root' or kind == 'Package'
        for name, sub_structure in structure.items():
            result[name] = _build_module_features_recursively(
                sub_structure, embedding_directory
            )
            result['@value'] = torch.mean(torch.stack([
                x['@value'] for x in result.values()
            ]))
        return result


def _flatten_features(raw):
    self = raw.pop('@value')
    if not raw:
        return {}   # Make sure classes are excluded
    result = {'': self}
    for key, value in raw.items():
        flattened = _flatten_features(value)
        result |= {
            (f'{key}.{k}' if k else key): v
            for k, v in flattened.items()
        }
    return result


################################################################################
################################################################################
# Pipeline
################################################################################


def pipeline(args: Config, logger: logging.Logger):
    mapping = dict(
        item.split('=')
        for item in args.project_legacy_mapping.split(';')
    )
    for project in os.listdir(args.structure_directory):
        if project in args.exclude_projects:
            logger.info('Skipping project %s', project)
            continue
        logger.info('Processing project %s', project)
        logger.info('Collecting version triples...')
        triples = get_version_triples(args.structure_directory / project)
        logger.info(f'Found {len(triples)} version triples.')
        for triple in triples:
            logger.debug(' * %s --> %s --> %s', *triple)
            for t in triple:
                logger.debug('Loading data for %s', t)
                version = '.'.join(map(str, t))
                structure_file, embedding_directory, graph_file = find_files_for_version(
                    project,
                    version,
                    graph_directory=args.graph_directory,
                    structure_directory=args.structure_directory,
                    embeddings_directory=args.embedding_directory,
                    legacy_mapping=mapping
                )
                feature_mapping = build_module_features(
                    structure_file, embedding_directory
                )
                # What now?
                # 1) Load the dependency graphs
                # 2) Derive the training and test labels from the dependency
                #    graphs, as specified by Tommasel et al.
                # 3) Derive the training and test edges from the dependency
                #    graphs
                # 4) Train the GNN model
                # 5) Evaluate the GNN model
                # 7) Store run metrics



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
    os.makedirs(args.output_directory, exist_ok=True)
    logger = setup_logging(args.output_directory / 'log.txt')
    try:
        pipeline(args, logger)
    except Exception as e:
        logger.critical('=' * 99)
        logger.critical('An error occurred!')
        logger.critical('')
        logger.critical('')
        logger.critical('Error message: %s', e)
        logger.critical('')
        logger.critical('')
        logger.critical('Full traceback:')
        logger.critical('')
        tb = traceback.format_exception(e.__class__, e, e.__traceback__)
        for entry in tb:
            for line in entry.rstrip().splitlines():
                logger.critical(line.rstrip())
        logger.critical('=' * 99)
        raise e


if __name__ == '__main__':
    main(Config().parse_args())
