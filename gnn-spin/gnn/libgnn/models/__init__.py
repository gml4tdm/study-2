from hierarchical_gnn.base import HierarchicalGNN, HierarchicalGNNConfig


_registry = {}


def register(cls, config):
    _registry[cls.__name__] = (config, cls)


register(HierarchicalGNN, HierarchicalGNNConfig)
