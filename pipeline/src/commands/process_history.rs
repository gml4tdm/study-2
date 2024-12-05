use std::path::PathBuf;
use crate::processing::history::{convert_raw_history_data, Commit, FileChangeInfo};

pub fn process_history(in_file: PathBuf, out_file: PathBuf) -> anyhow::Result<()> {
    let file = std::fs::File::open(in_file)?;
    let reader = std::io::BufReader::new(file);
    let hist: Vec<Commit<FileChangeInfo>> = serde_json::from_reader(reader)?;
    let converted = convert_raw_history_data(hist)?;
    let file = std::fs::File::create(out_file)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &converted)?;
    Ok(())
}