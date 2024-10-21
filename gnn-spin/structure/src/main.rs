mod resolvers;

use std::path::{Path, PathBuf};
use crate::resolvers::{JavaLogicalFileNameResolver, LogicalFileNameResolver};

#[derive(Debug, serde::Serialize)]
struct Folder {
    name: String,
    #[serde(rename = "relative-path")] relative_path: String,
    files: Vec<SourceFile>,
    #[serde(rename = "sub-folders")] sub_folders: Vec<Folder>
}

#[derive(Debug, serde::Serialize)]
struct SourceFile {
    #[serde(rename = "physical-name")] physical_name: String,
    #[serde(rename = "logical-name")] logical_name: String,
}


fn collect_file_structure(root: impl AsRef<Path>) -> anyhow::Result<Folder> {
    let path = root.as_ref();
    if !path.is_dir() {
        return Err(anyhow::anyhow!("Path is not a directory"));
    }
    collect_file_structure_impl(path, PathBuf::from(".").as_path(), path)
}

fn collect_file_structure_impl(path: &Path,
                               relative_path: &Path, 
                               root: &Path) -> anyhow::Result<Folder> {
    let name = get_filename(path)?;
    let mut sub_folders = Vec::new();
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            files.push(SourceFile{
                physical_name: get_filename(path.as_path())?,
                logical_name: JavaLogicalFileNameResolver.resolve(
                    path.as_path(),
                    path.parent().expect("No parent"),
                    root
                )?
            });
        } else if path.is_dir() {
            sub_folders.push(
                collect_file_structure_impl(
                    path.as_path(), 
                    relative_path.join(get_filename(path.as_path())?).as_path(),
                    root
                )?
            )
        } else {
            println!("WARNING: {} is neither a file nor a directory", path.display());
        }
    }
    let f = Folder { 
        name,
        sub_folders, 
        files, 
        relative_path: relative_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("Could not convert filename to string"))?
            .to_string()
    };
    Ok(f)
}

fn get_filename(path: &Path) -> anyhow::Result<String> {
    let name = path.file_name()
        .ok_or_else(|| anyhow::anyhow!("Could not get filename"))?
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("Could not convert filename to string"))?
        .to_string();
    Ok(name)
}


fn main() -> anyhow::Result<()> {
    println!("WARNING: This code should be updated for non-Java projects");
    let input_directory = std::env::args().nth(1)
        .ok_or_else(|| anyhow::anyhow!("No input directory provided"))?;
    let output_directory = PathBuf::from(
        std::env::args().nth(2)
            .ok_or_else(|| anyhow::anyhow!("No output directory provided"))?
    );
    std::fs::create_dir_all(output_directory.as_path())?;
    for entry in std::fs::read_dir(input_directory)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let project_output_dir = output_directory.as_path()
            .join(get_filename(path.as_path())?);
        std::fs::create_dir_all(project_output_dir.as_path())?;
        for inner_entry in std::fs::read_dir(path)? {
            let inner_entry = inner_entry?;
            let inner_path = inner_entry.path();
            if !inner_path.is_dir() {
                continue;
            }
            let structure = collect_file_structure(inner_path)?;
            let result_name = project_output_dir.as_path()
                .join(format!("{}.json", structure.name));
            let file = std::fs::File::create(result_name)?;
            serde_json::to_writer_pretty(file, &structure)?;
        }
    }
    Ok(())
}
