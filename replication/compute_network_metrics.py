import json
import math
import os
import re
import xml.etree.ElementTree

import alive_progress
import networkx
import numpy
import polars
from alive_progress import alive_bar

DATA_DIRECTORY = 'data'


def main():
    if not os.path.exists(DATA_DIRECTORY):
        raise ValueError('Data directory does not exist. Be sure to run `prepare_data.py` first!')
    for filename in os.listdir(DATA_DIRECTORY):
        path = os.path.join(DATA_DIRECTORY, filename)
        if not os.path.isdir(path):
            continue
        for file in os.listdir(path):
            full_path = os.path.join(path, file)
            if not file.endswith('.odem'):
                continue
            print('Processing {}'.format(file))
            metrics = generate_metrics(full_path)
            with open(full_path.replace('.odem', '.json'), 'w') as f:
                json.dump(metrics, f, indent=2)


def generate_metrics(filename):
    df = polars.read_csv(_find_text_features(filename), separator=';')
    semantic_map = {}
    for row in df.rows(named=True):
        key = (row.pop('class1'), row.pop('class2'))
        semantic_map[key] = row
    graph = Graph.from_xml(filename)
    data = {
        'nodes': list(graph.nodes),
        'edges': [
            {'from': key[0], 'to': key[1]}
            for key in graph.edges
        ],
        'link-features': [],
        'links-without-semantics': [],
        'links-without-topology': []
    }
    n = len(graph.nodes)
    total = n**2 - n

    def _is_ignored(z):
        return z.startswith(('java.', 'javax.', 'sun.'))
    with alive_progress.alive_bar(total) as progress:
        for x in graph.nodes:
            #if graph.nodes[x] is None:
            #    continue
            for y in graph.nodes:
                #if graph.nodes[x] is None:
                #    continue
                if x == y:
                    continue
                if (x, y) not in semantic_map:
                    if not _is_ignored(x) and not _is_ignored(y):
                        data['links-without-semantics'].append({'from': x, 'to': y})
                    progress()
                    continue
                entry = {
                    'from': x,
                    'to': y,
                    'topological-features': {
                        'common_neighbours': graph.common_neighbours(x, y),
                        'salton': graph.salton(x, y),
                        'sorensen': graph.sorensen(x, y),
                        'adamic_adar': graph.adamic_adar(x, y),
                        'katz': graph.katz(x, y),
                        'sim_rank': graph.sim_rank(x, y),
                        'russel_rao': graph.russel_rao(x, y),
                        'resource_allocation': graph.resource_allocation(x, y),
                    },
                    'semantic-features':  semantic_map.pop((x, y))
                }
                data['link-features'].append(entry)
                progress()
    data['links-without-topology'] = [
        (x, y) for x, y in semantic_map if not _is_ignored(x) and not _is_ignored(y)
    ]
    return data


class Graph:

    def __init__(self, spec):
        self.nodes = set()
        self.edges = set()
        # for (node, node_type), edges in spec.items():
        #     self.nodes[node] = node_type
        #     for (target, edge_type) in edges:
        #         if target not in self.nodes:
        #             self.nodes[target] = None
        #         self.edges[(node, target)] = edge_type
        # no_types = []
        # for node, node_type in self.nodes.items():
        #     if node_type is None and not node.startswith('java.'):
        #         no_types.append(node)
        # if no_types:
        #     # raise ValueError(f'The following nodes have no type: {no_types}')
        #     pass
        for node, edges in spec.items():
            self.nodes.add(node)
            for edge in edges:
                self.nodes.add(edge)
                self.edges.add((node, edge))
        self.graph = networkx.Graph()
        self.graph.add_nodes_from(self.nodes)
        self.graph.add_edges_from(self.edges)
        self._katz = None
        self._sim = None


    @classmethod
    def from_xml(cls, filename):
        tree = xml.etree.ElementTree.parse(filename)
        root = tree.getroot()
        context = cls._find_one_tag(root, 'context')
        container = cls._find_one_tag(context, 'container')
        nodes = {}
        for namespace in container:
            assert namespace.tag == 'namespace'
            for child in namespace:
                assert child.tag == 'type'
                source = child.get('name')
                source_type = child.get('classification')
                assert source is not None
                assert source_type is not None
                dependencies = cls._find_one_tag(child, 'dependencies')
                for dep in dependencies:
                    assert dep.tag == 'depends-on'
                    target = dep.get('name')
                    edge_type = dep.get('classification')
                    assert target is not None
                    assert edge_type is not None
                    nodes.setdefault((source, source_type), []).append((target, edge_type))
        return cls(cls._lift_to_packages(nodes))

    @staticmethod
    def _find_one_tag(tag, name):
        child = None
        for c in tag:
            if c.tag == name:
                if child is not None:
                    raise ValueError(f'Duplicate child {name}')
                child = c
        return child

    @staticmethod
    def _lift_to_packages(cls_diagram):
        package_diagram = {}
        for (node, _), edges in cls_diagram.items():
            src_cls = '.'.join(node.split('.')[:-1])
            for to, _ in edges:
                to_cls = '.'.join(to.split('.')[:-1])
                package_diagram.setdefault(src_cls, set()).add(to_cls)
        return package_diagram

    def common_neighbours(self, node_a, node_b):
        return len(set(networkx.common_neighbors(self.graph, node_a, node_b)))

    def salton(self, node_a, node_b):
        # https://www-sciencedirect-com.proxy-ub.rug.nl/science/article/pii/S037843711000991X
        x = self.graph.neighbors(node_a)
        y = self.graph.neighbors(node_b)
        z = set(x) & set(y)
        dx = self.graph.degree(node_a)
        dy = self.graph.degree(node_b)
        dz = math.sqrt(dx * dy)         # type: ignore
        return len(z) / dz

    def sorensen(self, node_a, node_b):
        # https://www-sciencedirect-com.proxy-ub.rug.nl/science/article/pii/S037843711000991X
        x = self.graph.neighbors(node_a)
        y = self.graph.neighbors(node_b)
        z = set(x) & set(y)
        dx = self.graph.degree(node_a)
        dy = self.graph.degree(node_b)
        return 2 * len(z) / (dx + dy)   # type: ignore

    def adamic_adar(self, node_a, node_b):
        items = list(networkx.adamic_adar_index(self.graph, [(node_a, node_b)]))
        return items[0][-1]

    def katz(self, node_a, node_b):
        # Based on
        # https://evalne.readthedocs.io/en/latest/_modules/evalne/methods/katz.html#Katz
        if self._katz is None:
            adj = networkx.adjacency_matrix(self.graph)
            beta = 0.005
            aux = adj.T.multiply(-beta).todense()
            numpy.fill_diagonal(aux, 1 + aux.diagonal())
            sim = numpy.linalg.inv(aux)
            numpy.fill_diagonal(sim, sim.diagonal() - 1)
            self._katz = sim
        nodes = list(self.graph.nodes())
        i = nodes.index(node_a)
        j = nodes.index(node_b)
        assert abs(self._katz[i, j] - self._katz[j, i]) <= 1e-4
        return self._katz[i, j]

    def sim_rank(self, node_a, node_b):
        #return networkx.simrank_similarity(self.graph, source=node_a, target=node_b)
        if self._sim is None:
            self._sim = networkx.simrank_similarity(self.graph)
        return self._sim[node_a][node_b]

    def russel_rao(self, node_a, node_b):
        # based on decompiled version of the code of Tomassel et al.
        x = self.graph.neighbors(node_a)
        y = self.graph.neighbors(node_b)
        z = set(x) & set(y)
        return len(z) / len(self.graph.nodes)

    def resource_allocation(self, node_a, node_b):
        items = list(networkx.resource_allocation_index(self.graph, [(node_a, node_b)]))
        return items[0][-1]


def _find_text_features(path):
    directory, original_filename = os.path.split(path)
    prefix = re.match(r'[a-zA-Z_\-0-9]+-\d+(\.\d+)*', original_filename).group()
    for filename in os.listdir(directory):
        if filename.startswith(prefix) and filename.endswith('.txt') and not filename.removeprefix(prefix)[0].isdigit():
            return os.path.join(directory, filename)
    raise ValueError(f'Could not find semantic features for file {path}')



if __name__ == '__main__':
    main()
