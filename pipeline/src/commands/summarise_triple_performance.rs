use std::collections::HashMap;
use std::path::PathBuf;
use crate::replication::as_predictor::developer::ASPredictorRun;
use crate::utils::metrics::{BinaryClassificationMetrics, BinaryConfusionMatrix};
use crate::utils::paths::ExtractFileName;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TriplePerformance {
    scores: Vec<TriplePerformanceScore>
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct TriplePerformanceScore {
    project: String,
    versions: (String, String, String),
    accuracy: f64,
    precision: f64,
    recall: f64,
    f1: f64,
    balanced_accuracy: f64,
    cohen_kappa: f64,
    true_positives: u64,
    true_negatives: u64,
    false_positives: u64,
    false_negatives: u64
}


pub fn summarise_triple_performance(input_files: Vec<PathBuf>,
                                    output_directory: PathBuf) -> anyhow::Result<()>
{
    if !output_directory.exists() {
        std::fs::create_dir_all(output_directory.as_path())?;
    }
    let metrics_by_file = get_metrics_by_file(&input_files)?;

    for (path, metrics) in input_files.into_iter().zip(metrics_by_file) {
        let out_path = output_directory.join(path.extract_filename());
        let out_metrics = metrics.into_iter()
            .map(|(k, v)| {
                let (project, v1, v2, v3) = k;
                TriplePerformanceScore {
                    project,
                    versions: (v1, v2, v3),
                    accuracy: v.accuracy(),
                    precision: v.precision(),
                    recall: v.recall(),
                    f1: v.f1_score(),
                    balanced_accuracy: v.balanced_accuracy(),
                    cohen_kappa: v.cohen_kappa(),
                    true_positives: v.confusion_matrix.true_positives,
                    true_negatives: v.confusion_matrix.true_negatives,
                    false_positives: v.confusion_matrix.false_positives,
                    false_negatives: v.confusion_matrix.false_negatives
                }
            })
            .collect::<Vec<_>>();
        let out = TriplePerformance { scores: out_metrics };
        let file = std::fs::File::create(out_path)?;
        let writer = std::io::BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &out)?;
    }

    Ok(())
}

fn get_metrics_by_file(files: &[PathBuf]) -> anyhow::Result<Vec<HashMap<(String, String, String, String), BinaryClassificationMetrics>>> {
    let metrics_by_file = files.iter()
        .map(|filename| {
            let file = std::fs::File::open(filename)?;
            let reader = std::io::BufReader::new(file);
            let runs: Vec<ASPredictorRun> = serde_json::from_reader(reader)?;
            let metrics_for_run = runs.into_iter()
                .filter(|run| run.output.is_some())
                .map(|run| {
                    let out = run.output.unwrap();
                    (
                        (
                            run.project.clone(),
                            run.version_1.clone(),
                            run.version_2.clone(),
                            run.version_3.clone()
                        ),
                        BinaryClassificationMetrics::from_confusion_matrix(
                            BinaryConfusionMatrix::from_counts(
                                out.true_positives, out.false_positives,
                                out.true_negatives, out.false_negatives
                            )
                        )
                    )
                })
                .collect::<HashMap<_, _>>();
            Ok(metrics_for_run)
        })
        .collect::<Result<Vec<_>, anyhow::Error>>()?;
    Ok(metrics_by_file)
}
