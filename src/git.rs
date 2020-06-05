use crate::api::search::PullRequestStatus;
use crate::graph::FlatDep;

fn process_ref(git_ref: &str) -> String {
    git_ref.replace("heap:", "")
}

/// For all open pull requests in the graph, generate a series of commands
/// (force-pushes) that will rebase the entire stack. The "PREBASE" variable
/// is a base for the first branch in the stack (essentially a "stop cherry-picking
/// here" marker), and is required because of our squash-merge workflow.
/// TODO: Move this directly into Rust.
pub fn generate_rebase_script(deps: FlatDep) -> String {
    let deps = deps
        .iter()
        .filter(|(dep, _)| *dep.state() == PullRequestStatus::Open)
        .collect::<Vec<_>>();

    let mut out = String::new();

    out.push_str("#!/usr/bin/env bash\n\n");
    out.push_str("set -euo pipefail\n");
    out.push_str("set -o xtrace\n\n");

    out.push_str("# ------ THIS SCRIPT ASSUMES YOUR PR STACK IS A SINGLE CHAIN WITHOUT BRANCHING ----- #\n\n");
    out.push_str("# It starts at the base of the stack, cherry-picking onto the new base and force-pushing as it goes.\n");
    out.push_str("# We can't tell where the initial cherry-pick should stop (mainly because of our squash merge workflow),\n");
    out.push_str(
        "# so that initial stopping point for the first PR needs to be specified manually.\n\n",
    );

    out.push_str("export PREBASE=\"<enter a marker to stop the initial cherry-pick at>\"\n");

    for (from, to) in deps {
        let to = if let Some(pr) = to {
            pr.head().to_string()
        } else {
            String::from("<enter a ref to rebase the stack on; usually `develop`>")
        };

        out.push_str("\n# -------------- #\n\n");

        out.push_str(&format!("export TO=\"{}\"\n", process_ref(&to)));
        out.push_str(&format!("export FROM=\"{}\"\n\n", process_ref(from.head())));

        out.push_str("git checkout heap/\"$TO\"\n");
        out.push_str("git cherry-pick \"$PREBASE\"..heap/\"$FROM\"\n");
        out.push_str("export PREBASE=\"$(git rev-parse --verify heap/$FROM)\"\n");
        out.push_str("git push -f heap HEAD:refs/heads/\"$FROM\"\n");
    }

    out
}
