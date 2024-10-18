import pydantic
import torch

from ..config_resolution import ModelConfigResolver


class FNNLayerSpec(pydantic.BaseModel):
    name: str
    output_size: int


class MLPConfig(ModelConfigResolver, pydantic.BaseModel):
    fnn_layers: list[FNNLayerSpec]

    def get_mlp(self, input_size: int) -> list[torch.nn.Module]:
        result = []
        for spec in self.gnn_layers:
            result.append(
                self.resolve_fnn_layer(spec.name, input_size, spec.output_size)
            )
            input_size = spec.output_size
        return result

    @property
    def output_size(self) -> int:
        return self.fnn_layers[-1].output_size


class MLP(torch.nn.Module):

    def __init__(self, input_dim: int, config: MLPConfig):
        super().__init__()
        self.layers = config.get_mlp(input_dim)

    @property
    def output_size(self) -> int:
        return self.config.output_size

    def forward(self, *x):
        for layer in self.layers:
            x = layer(*x)
        return x
