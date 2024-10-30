from __future__ import annotations

import json
import typing

import pydantic_xml


FILENAME = 'apache-camel-2.0.0-src_apache-camel-2.1.0-src-predicted_dependendencies_smo-rbf.xml'


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



class dependencies(pydantic_xml.BaseXmlModel):
    dependencies: list[Dependency] = pydantic_xml.element(tag='dependency',
                                                          default=[])


class Dependency(pydantic_xml.BaseXmlModel):
    source: str = pydantic_xml.element(tag='source')
    target: str = pydantic_xml.element(tag='target')
    confidence: float = pydantic_xml.element(tag='confidence')


dependencies.model_rebuild()


def main():
    with open(FILENAME) as f:
        xml = f.read()

    prediction = dependencies.from_xml(xml)

    print(len(prediction.dependencies))

    with open('predictions.json') as f:
        data = json.load(f)

    print(len(data['predictions']))

    vertices_1 = set()
    for dep in prediction.dependencies:
        vertices_1.add(dep.source)
        vertices_1.add(dep.target)

    vertices_2 = set()
    for start, stop in data['names']:
        vertices_2.add(start)
        vertices_2.add(stop)

    print('Vertices 1:', len(vertices_1))
    print('Vertices 2:', len(vertices_2))
    print('Shared vertices: ', len(vertices_1 & vertices_2))
    print('Total vertices: ', len(vertices_1 | vertices_2))
    print('Unique Vertices in 1:', len(vertices_1 - vertices_2))
    print('Unique Vertices in 2:', len(vertices_2 - vertices_1))

    with open('apache-camel-2.0.0.odem') as f:
        odem = ODEM.from_xml(f.read())

    vertices_3 = set()
    for context in odem.contexts:
        for container in context.containers:
            for namespace in container.namespaces:
                vertices_3.add(namespace.name)
                for type_ in namespace.types:
                    for dependency in type_.dependencies.depends_on:
                        dependency_package = dependency.name.rsplit(
                            '.', maxsplit=1
                        )[0]
                        if dependency_package in vertices_3:
                            vertices_3.add(dependency_package)

    print('Vertices 3:', len(vertices_3))
    print('Shared vertices: ', len(vertices_3 & vertices_1))
    print('Total vertices: ', len(vertices_3 | vertices_1))
    print('Unique Vertices in 1:', len(vertices_1 - vertices_3))
    print('Unique Vertices in 3:', len(vertices_3 - vertices_1))

    print(vertices_1 - vertices_3)
    print(vertices_3 - vertices_1)


if __name__ == '__main__':
    main()
