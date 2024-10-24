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
    #[serde(rename = "logical-units")] logical_units: Vec<LogicalNameInfo>,
}

#[derive(Debug, serde::Serialize)]
struct LogicalNameInfo {
    #[serde(rename = "name")] name: String,
    #[serde(rename = "type")] r#type: String,
    #[serde(rename = "byte-start")] byte_start: Option<usize>,
    #[serde(rename = "byte-stop")] byte_stop: Option<usize>
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
            let resolved = JavaLogicalFileNameResolver.resolve(
                path.as_path(),
                path.parent().expect("No parent"),
                root
            )?;
            files.push(SourceFile{
                physical_name: get_filename(path.as_path())?,
                logical_units: resolved.into_iter().map(
                    |(name, kind, span)| LogicalNameInfo {
                        name,
                        r#type: kind,
                        byte_start: span.map(|(start, _)| start),
                        byte_stop: span.map(|(_, stop)| stop)
                    }
                ).collect()
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
    let writer = logforth::append::rolling_file::RollingFileWriter::builder()
        .max_log_files(1)
        .rotation(logforth::append::rolling_file::Rotation::Never)
        .build("logs")?;
    let (nonblocking, _guard) = logforth::append::rolling_file::NonBlockingBuilder::default()
        .finish(writer);
    let file = logforth::append::rolling_file::RollingFile::new(nonblocking);
    logforth::Logger::new()
        .dispatch(
            logforth::Dispatch::new()
                .filter(log::LevelFilter::Warn)
                .layout(logforth::layout::TextLayout::default())
                .append(logforth::append::Stdout)
        )
        .dispatch(
            logforth::Dispatch::new()
                .filter(log::LevelFilter::Debug)
                .layout(logforth::layout::TextLayout::default().no_color())
                .append(file)
        )
        .apply()?;

    log::warn!("WARNING: This code should be updated for non-Java projects");
    
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
