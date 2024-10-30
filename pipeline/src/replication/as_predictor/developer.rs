use std::io::{BufRead, Lines};
use std::iter::Peekable;
use std::path::Path;
use std::sync::OnceLock;
use itertools::Itertools;

#[derive(Debug, Clone)]
#[derive(serde::Serialize, serde::Deserialize)]
pub struct ASPredictorRun {
    pub project: String,
    pub version_1: String,
    pub version_2: String,
    pub version_3: String,
    pub output: Option<ASPredictorOutput>,
}


#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ASPredictorOutput {
    pub predicted_dependencies: Vec<(String, String)>,
    pub true_positives: u64,
    pub false_positives: u64,
    pub false_negatives: u64,
    pub true_negatives: u64,
}


pub fn read_as_predictor_output(path: impl AsRef<Path>) -> anyhow::Result<Vec<ASPredictorRun>> {
    log::info!("Converting {}...", path.as_ref().display());
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut stream = reader.lines().peekable();
    let _header = stream.next().ok_or(anyhow::anyhow!("Empty output file"))??;
    let mut result = Vec::new();
    while let Some(run) = read_run(&mut stream)? {
        result.push(run);
    }
    Ok(result)
}

static RUN_HEADER_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
static CONFUSION_ROW_1_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
static CONFUSION_ROW_2_PATTERN: OnceLock<regex::Regex> = OnceLock::new();
static PREDICTIONS_PATTERN: OnceLock<regex::Regex> = OnceLock::new();

fn get_run_header_pattern() -> &'static regex::Regex {
    RUN_HEADER_PATTERN.get_or_init(|| regex::Regex::new(r"(?x)
                (?<proj>[a-zA-Z\-_0-9]+)-(?<v1>([a-zA-Z\-_0-9]+)(\.[a-zA-Z\-_0-9]+)*)\s\#\#\s
                [a-zA-Z\-_0-9]+-(?<v2>([a-zA-Z\-_0-9]+)(\.[a-zA-Z\-_0-9]+)*)\s\#\#\s
                [a-zA-Z\-_0-9]+-(?<v3>([a-zA-Z\-_0-9]+)(\.[a-zA-Z\-_0-9]+)*)"
    ).unwrap())
}

fn get_confusion_row_1_pattern() -> &'static regex::Regex {
    CONFUSION_ROW_1_PATTERN.get_or_init(|| regex::Regex::new(r"(?x)
        \s*(?<tp>\d+)\s+(?<fn>\d+)\s+\|\s+a\s=\strue"
    ).unwrap())
}

fn get_confusion_row_2_pattern() -> &'static regex::Regex {
    CONFUSION_ROW_2_PATTERN.get_or_init(|| regex::Regex::new(r"(?x)
        \s*(?<fp>\d+)\s+(?<tn>\d+)\s+\|\s+b\s=\sfalse"
    ).unwrap())
}

fn get_predictions_pattern() -> &'static regex::Regex {
    PREDICTIONS_PATTERN.get_or_init(|| regex::Regex::new(r"(?x)
        (?<count>\d+)\s---\s\[(?<items>[a-zA-Z0-9_.]+-[a-zA-Z0-9_.]+(,\s[a-zA-Z0-9_.]+-[a-zA-Z0-9_.]+)*)"
    ).unwrap())
}

fn read_run<I>(stream: &mut Peekable<Lines<I>>) -> anyhow::Result<Option<ASPredictorRun>>
where
    I: BufRead,
{
    let opt = parse_run_header(stream)?;
    let mut run = match opt {
        Some(run) => run,
        None => return Ok(None),
    };
    let separator = stream.next()
        .ok_or(anyhow::anyhow!("Unexpected end of input"))??;
    if separator.as_str() != "outputScores" {
        return Err(anyhow::anyhow!("Expected `outputScores`"));
    }
    match stream.peek() {
        Some(line) => {
            let line = match line {
                Ok(x) => x,
                Err(_) => { 
                    let _ = stream.next().unwrap()?;
                    unreachable!("Expected error");
                }
            };
            let pattern = get_run_header_pattern();
            if pattern.is_match(line.as_str()) {
                return Ok(Some(run));
            }
        }
        None => { 
            return Ok(None); 
        }
    }
    let output = parse_confusion_matrix(stream)?;
    run.output = Some(output);
    // Skip to next or end 
    let pattern = get_run_header_pattern();
    while let Some(line) = stream.peek() {
        let line = match line {
            Ok(x) => x,
            Err(_) => {
                let _ = stream.next().unwrap()?;
                unreachable!("Expected error");
            }
        };
        if pattern.is_match(line.as_str()) {
            break;
        }
        let _ = stream.next().unwrap()?;
    }
    Ok(Some(run))
}

fn parse_run_header<I>(stream: &mut Peekable<Lines<I>>) -> anyhow::Result<Option<ASPredictorRun>>
where
    I: BufRead,
{
    let (project, version_1, version_2, version_3) = match stream.next() {
        Some(result) => {
            let line = result?;
            let pattern = get_run_header_pattern();
            let captures = pattern.captures(&line)
                .ok_or(anyhow::anyhow!("Could not parse header"))?;
            let project = captures.name("proj").unwrap().as_str().to_string();
            let version_1 = captures.name("v1").unwrap().as_str().to_string();
            let version_2 = captures.name("v2").unwrap().as_str().to_string();
            let version_3 = captures.name("v3").unwrap().as_str().to_string();
            log::info!(
                "Found project `{}` with version triple ({}, {}, {})",
                project, version_1, version_2, version_3
            );
            (project, version_1, version_2, version_3)
        }
        None => {
            return Ok(None);
        }
    };
    let run = ASPredictorRun { project, version_1, version_2, version_3, output: None, };
    Ok(Some(run))
}

fn parse_confusion_matrix<I>(stream: &mut Peekable<Lines<I>>) -> anyhow::Result<ASPredictorOutput>
where
    I: BufRead
{
    while let Some(line) = stream.next() {
        let line = line?;
        if line.as_str() == "=== Confusion Matrix ===" {
            break;
        }
    }
    // === Confusion Matrix ===
    // 
    // a  b   <-- classified as
    // 2  5 |  a = true
    // 26 72 |  b = false
    //
    let _ = stream.next().ok_or(anyhow::anyhow!("Unexpected end of input"))??;
    let _line = stream.next().ok_or(anyhow::anyhow!("Unexpected end of input"))??;
    // if line.as_str() != "  a  b   <-- classified as" {
    //     return Err(anyhow::anyhow!("Expected line to be '  a  b   <-- classified as', got '{}'", line));
    // }
    let line = stream.next().ok_or(anyhow::anyhow!("Unexpected end of input"))??;
    let pattern = get_confusion_row_1_pattern();
    let captures = pattern.captures(line.as_str())
        .ok_or(anyhow::anyhow!("Could not parse confusion matrix row 1 ({})", line))?;
    let tp = captures.name("tp").unwrap().as_str().parse::<u64>()?;
    let fn_ = captures.name("fn").unwrap().as_str().parse::<u64>()?;
    let line = stream.next().ok_or(anyhow::anyhow!("Unexpected end of input"))??;
    let pattern = get_confusion_row_2_pattern();
    let captures = pattern.captures(line.as_str())
        .ok_or(anyhow::anyhow!("Could not parse confusion matrix row 2 ({})", line))?;
    let fp = captures.name("fp").unwrap().as_str().parse::<u64>()?;
    let tn = captures.name("tn").unwrap().as_str().parse::<u64>()?;
    let _ = stream.next().ok_or(anyhow::anyhow!("Unexpected end of input"))??;
    let line = stream.next().ok_or(anyhow::anyhow!("Unexpected end of input"))??;
    let pattern = get_predictions_pattern();
    let captures = pattern.captures(line.as_str())
        .ok_or(anyhow::anyhow!("Could not parse predictions ({})", line))?;
    let count = captures.name("count").unwrap().as_str().parse::<usize>()?;
    let items = captures.name("items").unwrap().as_str().to_string();
    let predicted_dependencies = items.split(", ")
        .map(
            |x| x.split("-")
                .map(|s| s.to_string())
                .collect_tuple()
                .unwrap()
        )
        .collect::<Vec<_>>();
    log::info!(
        "Parsed run results (count = {}, tp = {}, fp = {}, fn = {}, tn = {})",
        count, tp, fp, fn_, tn
    );
    Ok(ASPredictorOutput { 
        predicted_dependencies, 
        true_positives: tp, 
        false_positives: fp,
        false_negatives: fn_,
        true_negatives: tn 
    })
}
