from __future__ import annotations

import pydantic


class PipelineConfig(pydantic.BaseModel):
    models: dict[str, ModelConfig]
    datasets: dict[str, DatasetConfig]


class DatasetConfig(pydantic.BaseModel):
    loader: DatasetLoaderConfig
    settings: dict[str, object]


class DatasetLoaderConfig(pydantic.BaseModel):
    name: str
    version: str


class ModelConfig(pydantic.BaseModel):
    version: str
    architecture: ModelArchitectureConfig
    training: ModelTrainingConfig
    metrics: list[object]
    dataset: str


class ModelArchitectureConfig(pydantic.BaseModel):
    builder: ModelBuilderConfig
    settings: dict[str, object]


class ModelBuilderConfig(pydantic.BaseModel):
    name: str
    version: str


class ModelTrainingConfig(pydantic.BaseModel):
    hyperparameters: dict[str, object]