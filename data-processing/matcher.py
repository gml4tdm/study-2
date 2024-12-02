import shared


def is_child_parent(graph: shared.Graph, edge: tuple[int, int]):
    x = graph.nodes[edge[0]].name
    y = graph.nodes[edge[1]].name
    return x.startswith(y + '.') or y.startswith(x + '.') or x == y
    # parent -> child should be implicit
    #return y.startswith(x + '.')
    #return x.startswith(y + '.')
    #return False

def main():
    filename = '../data/triples/apache-ant/apache-ant-1.3-1.4-1.5.json'
    triple = shared.VersionTriple.load_and_check(filename)
    graph = triple.test_graph

    count = 0
    for edge in graph.edge_labels.edges:
        if is_child_parent(graph, edge):
            continue
        count += 1

    print(count)


if __name__ == "__main__":
    main()
