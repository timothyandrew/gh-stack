use std::fs;

use crate::api::{PullRequestReviewState, PullRequestStatus};
use crate::graph::FlatDep;

pub fn build_table(deps: &FlatDep, title: &str, prelude_path: Option<&str>) -> String {
    let is_complete = deps
        .iter()
        .all(|(node, _)| node.state() == &PullRequestStatus::Closed);

    let mut out = String::new();

    if is_complete {
        out.push_str(&format!("### ✅ Stacked PR Chain: {}\n", title));
    } else {
        out.push_str(&format!("### Stacked PR Chain: {}\n", title));
    }

    if let Some(prelude_path) = prelude_path {
        let prelude = fs::read_to_string(prelude_path).unwrap();
        out.push_str(&prelude);
        out.push_str("\n");
    }

    out.push_str("| PR | Title | Status |  Merges Into  |\n");
    out.push_str("|:--:|:------|:-------|:-------------:|\n");

    for (node, parent) in deps {
        let review_state = match node.review_state() {
            PullRequestReviewState::APPROVED => "**Approved**",
            PullRequestReviewState::MERGED => "**Merged**",
            PullRequestReviewState::PENDING => "Pending",
            PullRequestReviewState::CHANGES_REQUESTED => "Changes requested",
            PullRequestReviewState::DISMISSED => "Dismissed",
            PullRequestReviewState::COMMENTED => "Commented",
        };

        let review_state = if node.review_state() != PullRequestReviewState::MERGED
            && *node.state() == PullRequestStatus::Closed
        {
            "**Closed**"
        } else {
            review_state
        };

        let row = match (node.state(), parent) {
            (_, None) => format!(
                "|#{}|{}|{}|{}|\n",
                node.number(),
                node.title(),
                review_state,
                "-"
            ),
            (_, Some(parent)) => format!(
                "|#{}|{}|{}|#{}|\n",
                node.number(),
                node.title(),
                review_state,
                parent.number(),
            ),
        };

        out.push_str(&row);
    }

    out
}
