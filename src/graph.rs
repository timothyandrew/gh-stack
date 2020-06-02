use std::collections::{HashMap, HashSet};

use crate::api::search::PullRequest;

pub fn build(prs: &[PullRequest]) {
    let mut heads = HashSet::new();
    let mut prs_by_base = HashMap::new();

    for pr in prs.iter() {
        heads.insert(pr.head());
        let entry = prs_by_base.entry(pr.base()).or_insert(Vec::new());
        entry.push(pr);
    }

    let roots: Vec<&PullRequest> = prs.iter().filter(|pr| !heads.contains(pr.base())).collect();
    let results = resolve(&roots, &prs_by_base);
}

fn resolve<'a>(
    roots: &Vec<&'a PullRequest>,
    prs_by_base: &'a HashMap<&str, Vec<&PullRequest>>
) -> Vec<&'a PullRequest> {
    let mut results = Vec::new();

    for &root in roots.iter() {
        results.push(root);
        if let Some(children) = prs_by_base.get(root.head()) {
            let mut children = resolve(children, prs_by_base);
            results.append(&mut children);
        }
    }
    
    results
}
