use crate::schema::{DependencyGraphRoot, DependsOn, Type};

impl DependencyGraphRoot {
    pub fn walk_graph<NodeFunction, EdgeFunction, NodeOutput, EdgeOutput>(
        &self,
        node_visitor: &NodeFunction, 
        edge_visitor: &EdgeFunction) -> (Vec<NodeOutput>, Vec<EdgeOutput>) 
    where 
        NodeFunction: Fn(&Type) -> NodeOutput,
        EdgeFunction: Fn(&Type, &DependsOn) -> EdgeOutput
    {
        let mut nodes = Vec::new();
        let mut edges = Vec::new();

        for container in &self.context.containers {
            for namespace in &container.namespaces {
                for r#type in &namespace.types {
                    nodes.push(node_visitor(r#type));
                    for dep in &r#type.dependencies.depends_on {
                        edges.push(edge_visitor(r#type, dep));
                    }
                }
            }
        }

        (nodes, edges)
    }
}
