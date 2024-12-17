use std::path::PathBuf;
use crate::datasets::timeseries::VersionTimeSeriesFeatures;


pub fn generate_time_series_features(graph_files: Vec<PathBuf>, output_file: PathBuf) -> anyhow::Result<()> {
    let features = VersionTimeSeriesFeatures::new(graph_files)?;
    let file = std::fs::File::create(output_file)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, &features)?;
    Ok(())
}
