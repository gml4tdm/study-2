use std::path::Path;
use crate::schema::DependencyGraphRoot;

mod schema;



fn load_graph(path: impl AsRef<Path>) -> anyhow::Result<DependencyGraphRoot> {
    let file = std::fs::File::open(path)?;
    let buffer = std::io::BufReader::new(file);
    let result = quick_xml::de::from_reader(buffer)?;
    Ok(result)
}

fn match_graphs(graph_1: DependencyGraphRoot,
                graph_2: DependencyGraphRoot,
                name_1: &str,
                name_2: &str) -> i32 {
    if graph_1.context.containers.len() != 1 {
        panic!("Graph 1 must have exactly one container");
    }
    if graph_2.context.containers.len() != 1 {
        panic!("Graph 2 must have exactly one container");
    }
    
    let mut differences = 0;

    let container_1 = &graph_1.context.containers[0];
    let container_2 = &graph_2.context.containers[0];

    for namespace_1 in &container_1.namespaces {
        let mut found_namespace = false;
        for namespace_2 in &container_2.namespaces {
            if namespace_1.name == namespace_2.name {
                found_namespace = true;

                //println!(" * Namespace {}", namespace_1.name);
                for r#type_1 in &namespace_1.types {
                    let mut found_type = false;
                    
                    //println!("    * Type {}", r#type_1.name);
                    for r#type_2 in &namespace_2.types {
                        if r#type_1.name == r#type_2.name {
                            found_type = true;
                            
                            for depends_1 in &r#type_1.dependencies.depends_on {
                                let mut found_depends = false;
                                for depends_2 in &r#type_2.dependencies.depends_on {
                                    if depends_1.name == depends_2.name {
                                        found_depends = true;
                                        if depends_1.classification != depends_2.classification {
                                            differences += 1;
                                            println!(" * In Namespace {}: In Type {}: Dependency {} from {} has different classification in {}",
                                                     namespace_1.name, r#type_1.name, depends_1.name, name_1, name_2);
                                        }
                                        
                                        break;
                                    }
                                }
                                
                                if !found_depends {
                                    differences += 1;
                                    println!(" * In Namespace {}: In Type {}: Dependency {} from {} not found in {}",
                                             namespace_1.name, r#type_1.name, depends_1.name, name_1, name_2);
                                }
                            }
                            
                            break;
                        }
                    }
                    if !found_type {
                        differences += 1;
                        println!(" * In Namespace {}: Type {} from {} not found in {}", 
                                 namespace_1.name, r#type_1.name, name_1, name_2);
                    }
                }
                
                break;
            }
        }
        if !found_namespace {
            differences += 1;
            println!(" * In Container: Namespace {} from {} not found in {}", namespace_1.name, name_1, name_2);
        }
    }
    
    differences 
}


fn main() -> anyhow::Result<()> {
    let filename_1 = std::env::args().nth(1)
        .expect("Please provide a file name");
    let filename_2 = std::env::args().nth(2)
        .expect("Please provide a file name");
    
    let graph_1 = load_graph(filename_1)?;
    let graph_2 = load_graph(filename_2)?;
    
    let x = match_graphs(graph_1.clone(), graph_2.clone(), "graph 1", "graph 2");
    let y = match_graphs(graph_2, graph_1, "graph 2", "graph 1");
    
    println!("Found {} differences", x + y);
    Ok(())
}
