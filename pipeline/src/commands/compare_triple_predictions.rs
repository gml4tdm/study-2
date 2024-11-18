use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::path::PathBuf;
use itertools::Itertools;
use prettytable::{Cell, Row, Table};
use crate::replication::as_predictor::developer::ASPredictorRun;
use crate::utils::metrics::{BinaryClassificationMetrics, BinaryConfusionMatrix};
use crate::utils::paths::ExtractFileName;

pub fn compare_triple_predictions_short(files: Vec<PathBuf>) -> anyhow::Result<()> {
    let metrics_by_file = get_metrics_by_file(&files)?;
    // Aggregate by project 
    let mut metrics_by_project_per_file = Vec::new();
    for metrics_for_file in metrics_by_file {
        let mut metrics_by_project: HashMap<String, Vec<BinaryClassificationMetrics>> = HashMap::new();
        for ((project, _, _, _), metrics) in metrics_for_file {
            metrics_by_project.entry(project).or_default().push(metrics);
        }
        metrics_by_project_per_file.push(metrics_by_project);
    }
    // Global Aggregate  
    let mut global_per_file = Vec::new();
    for metrics_by_project in metrics_by_project_per_file.iter() {
        let mut aggregated = Vec::new();
        for metrics in metrics_by_project.values() {
            aggregated.extend(metrics.iter().copied());
        }
        global_per_file.push(aggregated);
    }
    // Collect all projects 
    let mut projects = HashSet::new();
    for metrics_by_project in metrics_by_project_per_file.iter() {
        for project in metrics_by_project.keys() {
            projects.insert(project);
        }
    }
    let ordered = projects.into_iter().sorted().collect_vec();
    // Build table 
    let mut table = Table::new();
    let header = vec![Cell::new("Project"), Cell::new("Metrics Per File")];
    table.set_titles(Row::new(header));
    for project in ordered {
        let mut row = vec![Cell::new(project)];
        let mut inner_table = Table::new();
        let mut inner_header = files.iter()
            .map(|file| Cell::new(file.extract_filename()))
            .collect_vec();
        inner_header.insert(0, Cell::new("Metrics"));
        inner_table.set_titles(Row::new(inner_header));
        
        for (name, func) in get_metric_functions() {
            let mut inner_row = vec![Cell::new(name)];
            for metrics in metrics_by_project_per_file.iter() {
                if let Some(series) = metrics.get(project) {
                    let s = series.iter().map(&func).collect::<Vec<f64>>();
                    let mean = s.iter().sum::<f64>() / s.len() as f64;
                    let std_dev = s.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / s.len() as f64;
                    let content = format!("{mean:.4} \u{00B1} {std_dev:.4}");
                    inner_row.push(Cell::new(content.as_str()));
                } else {
                    inner_row.push(Cell::new(" "));
                }
            }
            inner_table.add_row(Row::new(inner_row));
        }
        
        row.push(Cell::new(inner_table.to_string().as_str()));
        table.add_row(Row::new(row));
    }

    let mut row = vec![Cell::new("Total")];
    let mut inner_table = Table::new();
    let mut inner_header = files.iter()
        .map(|file| Cell::new(file.extract_filename()))
        .collect_vec();
    inner_header.insert(0, Cell::new("Metrics"));
    inner_table.set_titles(Row::new(inner_header));

    for (name, func) in get_metric_functions() {
        let mut inner_row = vec![Cell::new(name)];
        for series in global_per_file.iter() {
            let s = series.iter().map(&func).collect::<Vec<f64>>();
            let mean = s.iter().sum::<f64>() / s.len() as f64;
            let std_dev = s.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / s.len() as f64;
            let content = format!("{mean:.4} \u{00B1} {std_dev:.4}");
            inner_row.push(Cell::new(content.as_str()));
        }
        inner_table.add_row(Row::new(inner_row));
    }

    row.push(Cell::new(inner_table.to_string().as_str()));
    table.add_row(Row::new(row));
    
    table.printstd();
    Ok(())
}

pub fn compare_triple_predictions(files: Vec<PathBuf>) -> anyhow::Result<()> {
    // Parse all metrics
    let metrics_by_file = get_metrics_by_file(&files)?;
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
            for (metric, func) in get_metric_functions() {
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

fn get_metric_functions() -> [(&'static str, Box<dyn Fn(&BinaryClassificationMetrics) -> f64>); 10] {
    let functions: [(&str, Box<dyn Fn(&BinaryClassificationMetrics) -> f64>); 10] = [
        ("accuracy", Box::new(BinaryClassificationMetrics::accuracy)),
        ("precision", Box::new(BinaryClassificationMetrics::precision)),
        ("recall", Box::new(BinaryClassificationMetrics::recall)),
        ("f1_score", Box::new(BinaryClassificationMetrics::f1_score)),
        ("balanced_accuracy", Box::new(BinaryClassificationMetrics::balanced_accuracy)),
        ("cohen_kappa", Box::new(BinaryClassificationMetrics::cohen_kappa)),
        ("true_positives", Box::new(|x| x.confusion_matrix.true_positives as f64)),
        ("false_positives", Box::new(|x| x.confusion_matrix.false_positives as f64)),
        ("true_negatives", Box::new(|x| x.confusion_matrix.true_negatives as f64)),
        ("false_negatives", Box::new(|x| x.confusion_matrix.false_negatives as f64)),
    ];
    functions 
}