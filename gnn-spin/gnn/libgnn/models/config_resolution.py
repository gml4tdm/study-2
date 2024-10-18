import abc

import torch
import torch_geometric


class ModelConfigResolver(abc.ABC):

    def __init__(self):
        pass

    @property
    @abc.abstractmethod
    def output_size(self) -> int:
        pass

    def resolve_graph_pooling(self,
                              name: str,
                              input_size: int,
                              **kwargs) -> torch.nn.Module:
        match name:
            case 'mean':
                return torch.nn.AvgPool1d(kernel_size=input_size, **kwargs)
            case _:
                raise ValueError(f'Graph pooling {name!r} not supported')

    def resolve_gnn_layer(self, name: str, input_dim: int,
                          output_dim: int, **kwargs) -> torch.nn.Module:
        match name:
            case 'gcn':
                return torch_geometric.nn.GCNConv(in_channels=input_dim,
                                                  out_channels=output_dim,
                                                  **kwargs)
            case _:
                raise ValueError(f'GNN layer {name!r} not supported')

    def resolve_fnn_layer(self, name: str, input_dim: int,
                          output_dim: int, **kwargs) -> torch.nn.Module:
        match name:
            case 'linear':
                return torch.nn.Linear(input_dim, output_dim)
            case _:
                raise ValueError(f'FNN layer {name!r} not supported')
