use error_set::error_set;

error_set!{
    LibrayError = GraphLibError;
    GraphLibError = GraphBuilderError || GraphError;
    GraphBuilderError = {
        #[display("Undefined vertex: {vertex}")]
        UndefinedVertex{vertex: String},
        #[display("Duplicate edge: {from_vertex} -> {to_vertex}")]
        DuplicateEdge{from_vertex: String, to_vertex: String},
        #[display("Duplicate vertex: {vertex}")]
        DuplicateVertex{vertex: String}
    };
    GraphError = {
        #[display("Undefined vertex: {vertex}")]
        UndefinedVertex{vertex: String},
    };
}
