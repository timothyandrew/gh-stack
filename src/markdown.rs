use regex::Regex;
use std::fs;

use crate::api::{PullRequestStatus, PullRequestReviewState};
use crate::graph::FlatDep;

fn process(row: String) -> String {
    // TODO: Make this configurable
    let regex = Regex::new(r"\[[^\]]+\]\s*").unwrap();
    regex.replace_all(&row, "").into_owned()
}

pub fn build_table(deps: &FlatDep, title: &str, prelude_path: Option<&str>) -> String {
    let is_complete = deps
        .iter()
        .all(|(node, _)| node.state() == &PullRequestStatus::Closed);

    let mut out = String::new();

    if let Some(prelude_path) = prelude_path {
        let prelude = fs::read_to_string(prelude_path).unwrap();
        out.push_str(&prelude);
        out.push_str("\n");
    }

    if is_complete {
        out.push_str(&format!("### ✅ Stacked PR Chain: {}\n", title));
    } else {
        out.push_str(&format!("### Stacked PR Chain: {}\n", title));
    }
    out.push_str("| PR | Title | Status |  Merges Into  |\n");
    out.push_str("|:--:|:------|:-------|:-------------:|\n");

    for (node, parent) in deps {
        let review_state = match node.review_state() {
            PullRequestReviewState::APPROVED => "**Approved**",
            PullRequestReviewState::PENDING => "Pending",
            PullRequestReviewState::CHANGES_REQUESTED => "Changes requested",
            PullRequestReviewState::DISMISSED => "Dismissed",
            PullRequestReviewState::COMMENTED => "Commented"
        };

        let row = match (node.state(), parent) {
            (_, None) => format!(
                "|#{}|{}|{}|{}|\n",
                node.number(),
                node.title(),
                review_state,
                "Base/Root"
            ),
            (PullRequestStatus::Closed, Some(parent)) => format!(
                "|#{}|{}|**Merged**|#{}|\n",
                node.number(),
                node.title(),
                parent.number()
            ),
            (_, Some(parent)) => format!(
                "|#{}|{}|{}|#{}|\n",
                node.number(),
                node.title(),
                review_state,
                parent.number(),
            ),
        };

        out.push_str(&process(row));
    }

    out
}
