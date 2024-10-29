use std::io::BufRead;
use std::path::Path;
use std::str::FromStr;

pub fn read_rsf_file<T, A, B, C, E1, E2, E3>(path: impl AsRef<Path>) -> anyhow::Result<Vec<T>>
where
    T: From<(A, B, C)>,
    A: FromStr<Err=E1>,
    B: FromStr<Err=E2>,
    C: FromStr<Err=E3>,
    E1: Into<anyhow::Error>,
    E2: Into<anyhow::Error>,
    E3: Into<anyhow::Error>
{
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut rsf = Vec::new();
    for line in reader.lines() {
        let line = line?;
        let part = line.split_whitespace().collect::<Vec<_>>();
        if part.len() != 3 {
            return Err(anyhow::anyhow!("Invalid RSF file"));
        }
        let a = A::from_str(part[0])
            .map_err(|e| e.into())?;
        let b = B::from_str(part[1])
            .map_err(|e| e.into())?;
        let c = C::from_str(part[2])
            .map_err(|e| e.into())?;
        rsf.push(T::from((a, b, c)));
    }
    Ok(rsf)
}
