use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use prettytable::{Cell, Row, Table};
use crate::replication::as_predictor::developer::ASPredictorRun;
use crate::utils::metrics::{BinaryClassificationMetrics, BinaryConfusionMatrix};

pub fn compare_triple_predictions(files: Vec<PathBuf>) -> anyhow::Result<()> {
    // Parse all metrics
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
    // Nice order for displaying projects
    let all_keys = metrics_by_file.iter()
        .flat_map(|x| x.keys())
        .collect::<HashSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut versions_by_project: HashMap<String, HashSet<(String, String, String)>> = HashMap::new();
    for (project, v1, v2, v3) in all_keys {
        let key = (v1.clone(), v2.clone(), v3.clone());
        match versions_by_project.entry(project.clone()) {
            Entry::Occupied(mut e) => {
                e.get_mut().insert(key);
            }
            Entry::Vacant(e) => {
                e.insert(HashSet::from([key]));
            }
        }
    }
    let filenames = files.iter()
        .map(|x| x.file_name().unwrap().to_str().unwrap())
        .collect::<Vec<_>>();
    let mut table = Table::new();
    // Set table headers
    let header = vec![
        Cell::new("Project"),
        Cell::new(" "),
    ];
    table.set_titles(Row::new(header));
    // Print projects
    for (project, versions) in versions_by_project {
        let mut row = vec![Cell::new(project.as_str())];
        let mut inner_table = Table::new();
        let inner_header = vec![Cell::new("Version"), Cell::new(" ")];
        //inner_header.extend(filenames.iter().map(|x| Cell::new(x)));
        inner_table.set_titles(Row::new(inner_header));

        let mut ordered = versions.into_iter().collect::<Vec<_>>();
        ordered.sort_by(
            |(v1, v2, v3), (u1, u2, u3)| {
                let c = crate::utils::versions::cmp_versions(v1, u1);
                if c != Ordering::Equal {
                    return c;
                }
                let c = crate::utils::versions::cmp_versions(v2, u2);
                if c != Ordering::Equal {
                    return c;
                }
                crate::utils::versions::cmp_versions(v3, u3)
            } 
        );
        for (v1, v2, v3) in ordered {
            let mut inner_row = vec![Cell::new(format!("{v1}, {v2}, {v3}").as_str())];

            let mut inner_inner_table = Table::new();
            let mut inner_inner_header = vec![Cell::new("Metrics")];
            inner_inner_header.extend(filenames.iter().map(|x| Cell::new(x)));
            inner_inner_table.set_titles(Row::new(inner_inner_header));
            let key = (project.clone(), v1.clone(), v2.clone(), v3.clone());
            let functions: [(&str, Box<dyn Fn(&BinaryClassificationMetrics) -> f64>); 8] = [
                ("accuracy", Box::new(BinaryClassificationMetrics::accuracy)),
                ("precision", Box::new(BinaryClassificationMetrics::precision)),
                ("recall", Box::new(BinaryClassificationMetrics::recall)),
                ("f1_score", Box::new(BinaryClassificationMetrics::f1_score)),
                ("true_positives", Box::new(|x| x.confusion_matrix.true_positives as f64)),
                ("false_positives", Box::new(|x| x.confusion_matrix.false_positives as f64)),
                ("true_negatives", Box::new(|x| x.confusion_matrix.true_negatives as f64)),
                ("false_negatives", Box::new(|x| x.confusion_matrix.false_negatives as f64)),
            ];
            for (metric, func) in functions.iter() {
                let mut inner_inner_row = vec![Cell::new(metric), ];
                for metrics_for_file in metrics_by_file.iter() {
                    match metrics_for_file.get(&key) {
                        Some(m) => {
                            inner_inner_row.push(Cell::new(format!("{:.4}", func(m)).as_str()))
                        }
                        None => {
                            inner_inner_row.push(Cell::new(" "));
                        }
                    }
                }
                inner_inner_table.add_row(Row::new(inner_inner_row));
            }
            inner_row.push(Cell::new(inner_inner_table.to_string().as_str()));
            inner_table.add_row(Row::new(inner_row));
        }
        row.push(Cell::new(inner_table.to_string().as_str()));
        table.add_row(Row::new(row));
    }
    table.printstd();
    Ok(())
}

