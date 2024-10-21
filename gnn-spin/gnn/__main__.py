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
# Structure Schema
################################################################################


class Folder(pydantic.BaseModel):
    name: str
    relative_path: pathlib.Path = pydantic.Field(alias='relative-path')
    files: list[SourceFile]
    sub_folders: list[Folder] = pydantic.Field(alias='sub-folders')


class SourceFile(pydantic.BaseModel):
    logical_name: str = pydantic.Field(alias='logical-name')
    physical_name: str = pydantic.Field(alias='physical-name')


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
    embedding_director = embeddings_directory / project / version
    inner_project = legacy_mapping.get(project, project)
    graph_file = graph_directory / project / f'{inner_project}-{version}.odem'
    return structure_file, embedding_director, graph_file


class VersionData:

    _structure_cache: dict[pathlib.Path, Folder] = {}
    _graph_cache = {}

    def __init__(self,
                 project: str,
                 version: str,
                 *,
                 structure_file: pathlib.Path,
                 embedding_directory: pathlib.Path,
                 graph_file: pathlib.Path,
                 logger: logging.Logger):
        self.project = project
        self.project_language = 'java'
        self.version = version
        self.structure_file = structure_file
        self.embedding_directory = embedding_directory
        self.graph_file = graph_file
        self.logger = logger
        self.logger.info('Loading data for %s - %s',
                         self.project, self.version)
        # Load graph data
        if self.graph_file not in self._graph_cache:
            with open(self.graph_file) as file:
                self._graph_cache[self.graph_file] = ODEM.from_xml(file.read())
        self.dependencies = self._graph_cache[self.graph_file]
        # Load structure data
        if self.structure_file not in self._structure_cache:
            with open(self.structure_file) as file:
                data = json.load(file)
                self._structure_cache[self.structure_file] = Folder(**data)
        self.structure = self._structure_cache[self.structure_file]
        self._classes_to_paths = self._map_packages_to_paths()

    def _map_packages_to_paths(self):
        # Create a mapping between packages and file system paths.
        # The mapping is not trivial because packages generally
        # do not have the full path as their prefix.
        package_to_path = {}
        for context in self.dependencies.contexts:
            for container in context.containers:
                for namespace in container.namespaces:
                    for tp in namespace.types:
                        package_to_path[tp.name] = self._find_file_in_structure(
                            namespace.name, tp.name.split('.')[-1]
                        )
        return package_to_path

    def _find_file_in_structure(self,
                                target_package: str,
                                target: str) -> pathlib.Path:
        #packages = self._find_package_in_structure(target_package)
        if self.project_language == 'java':
            if '$' in target:
                parent = target.rsplit('$')[0]
                self.logger.warning(
                    'Path and source code for inner type %s '
                    'will be resolved to that of its containing type %s',
                    target,
                    parent)
                target = parent
            if target.endswith('_'):
                target = target[:-1]
                self.logger.warning(
                    'Path and source code for type %s_ will be '
                    'resolved using the name %s',
                    target, target
                )

        results = list(self._search_structure(f'{target_package}.{target}'))
        if not results:
            message = f'Cannot find path for {target_package}.{target}'
            self.logger.critical(message)
            raise ValueError(message)
        if len(results) > 1:
            message = f'Multiple paths for {target_package}.{target}: {results}'
            self.logger.critical(message)
            raise ValueError(message)
        self.logger.debug('Found path for %s.%s: %s',
                          target_package, target, results[0])
        return results[0]

    def _search_structure(self, target: str):
        yield from self._search_structure_recursive(self.structure, target)


    def _search_structure_recursive(self, folder: Folder, target: str):
        for file in folder.files:
            if file.logical_name == target:
                yield folder.relative_path / file.physical_name
        for sub_folder in folder.sub_folders:
            yield from self._search_structure_recursive(sub_folder, target)


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
                structure_file, embedding_director, graph_file = find_files_for_version(
                    project,
                    version,
                    graph_directory=args.graph_directory,
                    structure_directory=args.structure_directory,
                    embeddings_directory=args.embedding_directory,
                    legacy_mapping=mapping
                )
                data = VersionData(
                    project=project,
                    version=version,
                    structure_file=structure_file,
                    embedding_directory=embedding_director,
                    graph_file=graph_file,
                    logger=logger
                )


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
