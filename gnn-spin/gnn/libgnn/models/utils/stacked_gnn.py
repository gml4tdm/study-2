import pydantic
import torch

from ..config_resolution import ModelConfigResolver

class GNNLayerSpec(pydantic.BaseModel):
    name: str
    output_size: int


class StackedGNNConfig(ModelConfigResolver, pydantic.BaseModel):
    gnn_layers: list[GNNLayerSpec]

    def get_gnn_stack(self, input_size: int) -> list[torch.nn.Module]:
        result = []
        for spec in self.gnn_layers:
            result.append(
                self.resolve_gnn_layer(spec.name, input_size, spec.output_size)
            )
            input_size = spec.output_size
        return result

    @property
    def output_size(self) -> int:
        return self.gnn_layers[-1].output_size


class StackedGNN(torch.nn.Module):

    def __init__(self, input_dim: int, config: StackedGNNConfig):
        super().__init__()
        self.gnn_layers = config.get_gnn_layers(input_dim)

    @property
    def output_size(self) -> int:
        return self.config.output_size

    def forward(self, *g):
        for layer in self.gnn_layers:
            g = layer(*g)
        return g
