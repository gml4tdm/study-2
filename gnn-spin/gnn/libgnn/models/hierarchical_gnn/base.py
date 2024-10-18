import abc

import torch

from ..config_resolution import ModelConfigResolver
from ..utils.stacked_gnn import StackedGNN, StackedGNNConfig
from ..utils.stacked_fnn import MLP, MLPConfig


class HierarchicalGNNConfig(ModelConfigResolver):
    initial_node_transform: MLPConfig = None
    bottom_up_gnn: StackedGNNConfig
    bottom_up_pooling: str
    module_gnn: StackedGNNConfig
    final_node_transform: MLPConfig = None

    def get_initial_node_transform(self,
                                   in_channels: int) -> torch.nn.Module | None:
        if self.initial_node_transform is None:
            return None
        return MLP(in_channels, self.initial_node_transform)

    def get_bottom_up_gnn(self, in_channels: int) -> torch.nn.Module:
        if self.initial_node_transform is not None:
            in_channels = self.initial_node_transform.output_size
        return StackedGNN(in_channels, self.bottom_up_gnn)

    def get_bottom_up_pooling(self) -> torch.nn.Module:
        return self.resolve_graph_pooling(
            self.bottom_up_pooling, self.bottom_up_gnn.output_size
        )

    def get_module_gnn(self):
        return StackedGNN(
            self.bottom_up_gnn.output_size, self.module_gnn
        )

    def get_final_node_transform(self) -> torch.nn.Module | None:
        if self.final_node_transform is None:
            return None
        return MLP(self.bottom_up_gnn.output_size, self.final_node_transform)

    @property
    def output_size(self):
        if self.final_node_transform is None:
            return self.module_gnn.output_size
        return self.final_node_transform.output_size


class HierarchicalGNN(abc.ABC, torch.nn.Module):

    def __init__(self, input_size: int, config: HierarchicalGNNConfig):
        super().__init__()
        self._initial_node_transform = config.get_initial_node_transform(input_size)
        self._bottom_up_gnn = config.get_bottom_up_gnn(input_size)
        self._bottom_up_pooling = config.get_bottom_up_pooling()
        self._module_gnn = config.get_module_gnn()
        self._final_node_transform = config.get_final_node_transform()

    def forward(self, x, edge_type, hierarchy):
        if self._initial_node_transform is not None:
            x = self._initial_node_transform(x)
        # TODO: How to handle homogeneous and heterogeneous GNN in a uniform manner?
        # TODO: Compute the correct graphs for bottom-up message passing
        x = self._bottom_up_gnn(x, edge_index, edge_type)
        x = self._bottom_up_pooling(x)
        # TODO: Compute the correct graph for message passing
        x = self._module_gnn(x, edge_index, edge_type)
        if self._final_node_transform is not None:
            x = self._final_node_transform(x)
        return x
