use std::cmp::Ordering;
use itertools::{EitherOrBoth, Itertools};

pub fn cmp_versions(a: &str, b: &str) -> Ordering {
    let lhs = a.split('.');
    let rhs = b.split('.');
    for pair in lhs.zip_longest(rhs) {
        match pair {
            EitherOrBoth::Both(x, y) => {
                let p = x.parse::<u64>();
                let q = y.parse::<u64>();
                match (p, q) {
                    (Ok(u), Ok(v)) if u < v => { return Ordering::Less; }
                    (Ok(u), Ok(v)) if u > v => { return Ordering::Greater; }
                    (Ok(_), Err(_)) => { return Ordering::Greater; }
                    (Err(_), Ok(_)) => { return Ordering::Less; }
                    (Err(_), Err(_)) => { 
                        let c = x.cmp(y);
                        if c != Ordering::Equal {
                            return c;
                        }
                    }
                    _ => {}
                }
            }
            EitherOrBoth::Left(_) => {
                return Ordering::Greater;
            }
            EitherOrBoth::Right(_) => {
                return Ordering::Less;
            }
        }
    }
    Ordering::Equal
}