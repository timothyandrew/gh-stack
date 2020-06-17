use regex::Regex;

use crate::api::search::PullRequestStatus;
use crate::graph::FlatDep;

fn process(row: String) -> String {
    // TODO: Make this configurable
    let regex = Regex::new(r"\[HEAP-\d+\]\s*").unwrap();
    regex.replace_all(&row, "").into_owned()
}

pub fn build_table(deps: FlatDep, title: &str) -> String {
    let is_complete = deps
        .iter()
        .all(|(node, _)| node.state() == &PullRequestStatus::Closed);

    let mut out = String::new();
    if is_complete {
        out.push_str(&format!("### ✅ Stacked PR Chain: {}\n", title));
    } else {
        out.push_str(&format!("### Stacked PR Chain: {}\n", title));
    }
    out.push_str("| PR | Title |  Merges Into  |\n");
    out.push_str("|:--:|:------|:-------------:|\n");

    for (node, parent) in deps {
        let row = match parent {
            Some(parent) => format!(
                "|#{}|{}|#{}|\n",
                node.number(),
                node.title(),
                parent.number()
            ),
            None => format!(
                "|#{}|{}|**{}**|\n",
                node.number(),
                node.title(),
                node.note()
            ),
        };

        out.push_str(&process(row));
    }

    out
}
