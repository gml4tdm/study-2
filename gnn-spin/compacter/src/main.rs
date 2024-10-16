use std::collections::HashSet;
use std::io::BufRead;
use std::path::Path;
use crate::schema::DependencyGraphRoot;

mod schema;
mod traversal;

fn convert_project_name(name: &str) -> &str {
    match name {
        "hibernate" => "hibernate-core",
        "apache-derby" => "db-derby",
        _ => name 
    }
}

fn main() -> anyhow::Result<()> {
    simple_logger::SimpleLogger::new().init()?;
    log::set_max_level(log::LevelFilter::Debug);

    let source_code_dir = std::path::PathBuf::from(
        std::env::args().nth(1).expect("no source code directory provided")
    );
    let graph_dir = std::path::PathBuf::from(
        std::env::args().nth(2).expect("no graph directory provided")
    );
    let output_dir = std::path::PathBuf::from(
        std::env::args().nth(3).expect("no output directory provided")
    );

    for entry in std::fs::read_dir(source_code_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            log::info!("Processing files in directory {}...", path.display());
            for inner_entry in std::fs::read_dir(path)? {
                let inner_entry = inner_entry?;
                let inner_path = inner_entry.path();
                if inner_path.is_dir() {
                    let version = inner_entry.file_name()
                        .to_str()
                        .expect("Invalid filename")
                        .to_string();
                    log::info!("Processing version {}...", version);
                    let project = entry.file_name().
                        to_str()
                        .expect("Invalid filename")
                        .to_string();
                    let graph_path = graph_dir.join(entry.file_name())
                        .join(format!("{}-{}.odem", convert_project_name(project.as_str()), version));
                    log::debug!("Looking for graph in file {}", graph_path.display());
                    let file = std::fs::File::open(graph_path)?;
                    let reader = std::io::BufReader::new(file);
                    let graph: DependencyGraphRoot = quick_xml::de::from_reader(reader)?;
                    let packages = graph.walk_graph(
                        &|node| node.name.rsplit_once('.').expect("Invalid package name").0.to_string(), 
                        &|_from, edge| edge.name.rsplit_once('.').expect("Invalid package name").0.to_string()
                    );
                    let unique_packages: HashSet<String> = packages.0.into_iter()
                        .chain(packages.1.into_iter())
                        .collect();
                    log::info!("Found {} unique packages", unique_packages.len());
                    log::info!("Copying package structure...");
                    let output_path = output_dir.join(entry.file_name())
                        .join(inner_entry.file_name());
                    let (included, ignored) = copy_package_structure(
                        inner_path, output_path.as_path(), &unique_packages
                    )?;
                    cleanup_empty_directories(output_path)?;
                    log::info!("Copied {} files (ignored {} Java files)", included, ignored);
                }
            }
        }
    }

    Ok(())
}


fn copy_package_structure(source: impl AsRef<Path>,
                          destination: impl AsRef<Path>, 
                          packages: &HashSet<String>) -> anyhow::Result<(i32, i32)> {
    let mut total = 0;
    let mut ignored = 0;
    std::fs::create_dir_all(destination.as_ref())?;
    for entry in std::fs::read_dir(source.as_ref())? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            let extension = match path.extension() {
                None => String::new(),
                Some(x) => x.to_str().expect("Invalid extension").to_string()
            };
            if extension != "java" {
                continue;
            } 
            // We get the package by searching for the first line 
            // which starts with the word "package" and ends with a semicolon
            let file = std::fs::File::open(&path);
            let reader = std::io::BufReader::new(file?);
            for line in reader.lines() {
                let line = line?.trim().to_string();
                if line.starts_with("package ") {
                    if !line.ends_with(";") {
                        panic!("Invalid package declaration: {}", line);
                    }
                    let package = line.strip_prefix("package")
                        .unwrap()
                        .strip_suffix(";")
                        .unwrap()
                        .trim();
                    log::trace!("Found file {} in package {}", path.display(), package);
                    if packages.contains(package) {
                        let destination_path = destination.as_ref().join(entry.file_name());
                        log::trace!("Copying file {} to {}", path.display(), destination_path.display());
                        std::fs::copy(&path, &destination_path)?;
                        total += 1;
                        break;
                    } else {
                        log::trace!("Ignoring file {} in package {}", path.display(), package);
                        ignored += 1;
                        break;
                    }
                }
            }
        } else if path.is_dir() {
            let destination_path = destination.as_ref().join(entry.file_name());
            let result = copy_package_structure(path, destination_path, packages)?;
            total += result.0;
            ignored += result.1;
        }
    }
    Ok((total, ignored))
}

fn cleanup_empty_directories(path: impl AsRef<Path>) -> anyhow::Result<()> {
    for entry in std::fs::read_dir(path.as_ref())? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            cleanup_empty_directories(path.as_path())?;
            if std::fs::read_dir(path.as_path())?.next().is_none() {
                std::fs::remove_dir(path)?;
            }
        }
    }
    Ok(())
}
