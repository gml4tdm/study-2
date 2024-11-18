################################################################################
################################################################################
# Imports
################################################################################

import json
import pathlib

import torch
import torch_geometric

import tap

import shared

################################################################################
################################################################################
# Data Preparation
################################################################################


def get_pytorch_dataset(graph: shared.Graph, *, dim=None, mapping=None):
    if dim is None:
        mapping = {}
        features = []
        for vertex in graph.nodes:
            i = len(features)
            mapping[vertex.name] = i
            features.append(i)

        dim = len(features)

        feat = torch.nn.functional.one_hot(
            torch.tensor(features, dtype=torch.int64)
        )
    else:
        features = []
        mask = []
        for vertex in graph.nodes:
            if vertex.name in mapping:
                features.append(mapping[vertex.name])
                mask.append(False)
            else:
                features.append(0)
                mask.append(True)

        feat = torch.nn.functional.one_hot(
            torch.tensor(features, dtype=torch.int64),
            num_classes=dim
        )

        feat[mask] = torch.zeros(dim, dtype=torch.long)

    data = torch_geometric.data.Data(
        x=feat.float(),
        edge_index=torch.tensor([
            [edge.from_ for edge in graph.edges],
            [edge.to for edge in graph.edges]
        ]),
        pred_edges=torch.tensor(graph.edge_labels.edges),
        y=torch.tensor(graph.edge_labels.labels, dtype=torch.float)
    )

    return data, dim, mapping


################################################################################
################################################################################
# Model
################################################################################


class Model2(torch.nn.Module):

    def __init__(self, embedding_in: int):
        super().__init__()
        conv = [128, 64, 32]
        linear = [16, 16]
        linear2 = [16, 8, 1]
        conv.insert(0, embedding_in)
        linear.insert(0, conv[-1])
        linear2.insert(0, linear[-1] * 2)
        self.conv = torch.nn.ModuleList([
            torch_geometric.nn.GCNConv(conv[i], conv[i + 1])
            for i in range(len(conv) - 1)
        ])
        self.linear = torch.nn.ModuleList([
            torch.nn.Linear(linear[i], linear[i + 1])
            for i in range(len(linear) - 1)
        ])
        self.linear2 = torch.nn.ModuleList([
            torch.nn.Linear(linear2[i], linear2[i + 1])
            for i in range(len(linear2) - 1)
        ])

    def forward(self, x):
        x, z = x.x, x
        for i in range(len(self.conv)):
            x = self.conv[i](x, z.edge_index)
            x = torch.relu(x)
        x = torch.flatten(x, 1)
        for i in range(len(self.linear)):
            x = self.linear[i](x)
            x = torch.relu(x)

        x = torch.flatten(x, 1)

        # Link prediction
        matrix = x[z.pred_edges]
        #pred = torch.mul(matrix[:, 0], matrix[:, 1])
        x = torch.concat([matrix[:, 0], matrix[:, 1]], dim=1)

        for i in range(len(self.linear2)):
            x = self.linear2[i](x)
            if i != len(self.linear2) - 1:
                x = torch.relu(x)

        pred = torch.sigmoid(x)
        return pred.transpose(0, 1).flatten(0)


class WeightedBCE:

    def __init__(self, class_weights: torch.Tensor, /):
        self._bce = torch.nn.BCELoss(reduction='none')
        self._class_weights = class_weights

    def __call__(self, input_: torch.Tensor, target: torch.Tensor):
        bce = self._bce(input_, target)
        weights = self._class_weights[(target >= 0.5).int()]
        return torch.mul(weights, bce).mean()



################################################################################
################################################################################
# Program Entrypoint
################################################################################


class Config(tap.Tap):
    input_files: list[pathlib.Path]
    output_file: pathlib.Path


def main(config: Config):
    results = []
    for filename in config.input_files:
        triple = shared.VersionTriple.load_and_check(filename)
        print(f'Loaded version triple from project {triple.project}: '
              f'{triple.version_1}, {triple.version_2}, {triple.version_3}')
        if not triple.metadata.gnn_safe:
            raise ValueError('Data not prepared for GNN')
        train, dim, mapping = get_pytorch_dataset(triple.training_graph)

        # Training
        device = 'cuda' if torch.cuda.is_available() else 'cpu'
        model = Model2(dim)
        model.to(device)
        train.to(device)
        optimizer = torch.optim.Adam(model.parameters(), lr=0.001)
        loss_fn = WeightedBCE(torch.tensor([0.05, 0.95]))
        for epoch in range(2500):
            out = model(train)
            loss = loss_fn(out, train.y)
            loss.backward()
            optimizer.step()
            optimizer.zero_grad()
            print(f'Epoch {epoch+1}: {loss}')

        # Evaluation
        del train
        test, _, _ = get_pytorch_dataset(triple.test_graph, dim=dim, mapping=mapping)
        with torch.no_grad():
            out = model(test)
            out = (out >= 0.5).tolist()
            result = shared.evaluate(triple, out)
            results.append(result)

    config.output_file.parent.mkdir(parents=True, exist_ok=True)
    with open(config.output_file, 'w') as file:
        json.dump(results, file, indent=2)


if __name__ == "__main__":
    main(Config().parse_args())
