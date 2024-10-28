#!/usr/bin/env python3
from __future__ import annotations

import sys
import typing

import graphviz
import pydantic_xml

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
# Rendering
################################################################################


def render_training_graph(old: ODEM, current: ODEM):
    vertices_old = set()
    for context in old.contexts:
        for container in context.containers:
            for namespace in container.namespaces:
                vertices_old.add(namespace.name)

    vertices_cur = set()
    for context in current.contexts:
        for container in context.containers:
            for namespace in container.namespaces:
                vertices_cur.add(namespace.name)

    edges_old = set()
    for context in old.contexts:
        for container in context.containers:
            for namespace in container.namespaces:
                for type_ in namespace.types:
                    for dependency in type_.dependencies.depends_on:
                        dependency_package = dependency.name.rsplit(
                            '.', maxsplit=1
                        )[0]
                        if dependency_package in vertices_old:
                            edges_old.add((namespace.name, dependency_package))

    edges_cur = set()
    for context in current.contexts:
        for container in context.containers:
            for namespace in container.namespaces:
                for type_ in namespace.types:
                    for dependency in type_.dependencies.depends_on:
                        dependency_package = dependency.name.rsplit(
                            '.', maxsplit=1
                        )[0]
                        if dependency_package in vertices_cur:
                            edges_cur.add((namespace.name, dependency_package))

    dot = graphviz.Digraph()
    for vertex in vertices_old - vertices_cur:
        dot.node(vertex, color='red')
    for vertex in vertices_cur - vertices_old:
        dot.node(vertex, color='green')
    for v in vertices_old & vertices_cur:
        dot.node(v, color='blue')

    for (x, y) in edges_old - edges_cur:
        dot.edge(x, y, color='red')
    for (x, y) in edges_cur - edges_old:
        dot.edge(x, y, color='green')
    for (x, y) in edges_old & edges_cur:
        dot.edge(x, y, color='blue')

    return dot

################################################################################
################################################################################
# Main
################################################################################



def main(old, current, new):
    with open(old) as file:
        odem_old = ODEM.from_xml(file.read())
    with open(current) as file:
        odem_current = ODEM.from_xml(file.read())
    with open(new) as file:
        odem_new = ODEM.from_xml(file.read())

    dot1 = render_training_graph(odem_old, odem_current)
    dot2 = render_training_graph(odem_current, odem_new)

    dot1.render('1.dot')
    dot2.render('2.dot')


if __name__ == '__main__':
    main(*sys.argv[1:])
