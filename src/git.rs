use crate::api::search::PullRequestStatus;
use crate::graph::FlatDep;
use crate::util::loop_until_confirm;
use git2::build::CheckoutBuilder;
use git2::{
    CherrypickOptions,
    Repository, Sort,
    Revwalk, Oid, Commit,
    Index
};

use std::error::Error;
use tokio::process::Command;

fn remote_ref(remote: &str, git_ref: &str) -> String {
    format!("{}/{}", remote, git_ref)
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

        out.push_str(&format!("export TO=\"{}\"\n", remote_ref("heap", &to)));
        out.push_str(&format!(
            "export FROM=\"{}\"\n\n",
            remote_ref("heap", from.head())
        ));

        out.push_str("git checkout \"$TO\"\n");
        out.push_str("git cherry-pick \"$PREBASE\"..\"$FROM\"\n");
        out.push_str("export PREBASE=\"$(git rev-parse --verify $FROM)\"\n");
        out.push_str("git push -f heap HEAD:refs/heads/\"$FROM\"\n");
    }

    out
}

fn oid_to_commit(repo: &Repository, oid: Oid) -> Commit {
    repo.find_commit(oid).unwrap()
}

fn head_commit(repo: &Repository) -> Commit {
    repo.find_commit(repo.head().unwrap().target().unwrap()).unwrap()
}

fn checkout_commit(repo: &Repository, commit: &Commit, options: Option<&mut CheckoutBuilder>) {
    repo.checkout_tree(&commit.as_object(), options).unwrap();
    repo.set_head_detached(commit.id()).unwrap();
}

fn rev_to_commit<'a>(repo: &'a Repository, rev: &str) -> Commit<'a> {
    let commit = repo.revparse_single(rev).unwrap();
    let commit = commit.into_commit().unwrap();
    commit
}

/// Commit and checkout `index`
fn create_commit<'a>(repo: &'a Repository, index: &mut Index, message: &str) -> Commit<'a> {
    let tree = index.write_tree_to(&repo).unwrap();
    let tree = repo.find_tree(tree).unwrap();

    let signature = repo.signature().unwrap();
    let commit = repo
        .commit(
            None,
            &signature,
            &signature,
            message,
            &tree,
            &[&head_commit(repo)]
        )
        .unwrap();

    let commit = oid_to_commit(&repo, commit);

    let mut cb = CheckoutBuilder::new();
    cb.force();

    checkout_commit(&repo, &commit, Some(&mut cb));

    // "Complete" the cherry-pick. There is likely a better way to do
    // this that I haven't found so far.
    repo.cleanup_state().unwrap();

    commit
}

fn cherry_pick_range(repo: &Repository, walk: &mut Revwalk) {
    for from in walk {
        let from = oid_to_commit(&repo, from.unwrap());

        if from.parent_count() > 1 {
            panic!("Exiting: I don't know how to deal with merge commits correctly.");
        }

        let mut cb = CheckoutBuilder::new();
        cb.allow_conflicts(true);
        let mut opts = CherrypickOptions::new();
        opts.checkout_builder(cb);

        println!("Cherry-picking: {:?}", from);
        repo.cherrypick(&from, Some(&mut opts)).unwrap();

        let mut index = repo.index().unwrap();

        if index.has_conflicts() {
            let prompt = "Conflicts! Resolve manually and `git add` each one (don't run any `git cherry-pick` commands, though).";
            loop_until_confirm(prompt);

            // Reload index from disk
            index = repo.index().unwrap();
            index.read(true).unwrap();
        }

        create_commit(&repo, &mut index, from.message().unwrap());
    }
}

pub async fn perform_rebase(
    deps: FlatDep,
    repo: &Repository,
    remote: &str,
    boundary: Option<&str>
) -> Result<(), Box<dyn Error>> {
    let deps = deps
        .iter()
        .filter(|(dep, _)| *dep.state() == PullRequestStatus::Open)
        .collect::<Vec<_>>();

    let (pr, _) = deps[0];

    let base = rev_to_commit(&repo, &remote_ref(remote, pr.base()));
    let head = rev_to_commit(&repo, pr.head());

    let mut stop_cherry_pick_at = match boundary {
        Some(rev) => rev_to_commit(&repo, rev).id(),
        None => repo.merge_base(base.id(), head.id()).unwrap()
    };
    let mut update_local_branches_to = vec![];

    println!("Checking out {:?}", base);
    checkout_commit(&repo, &base, None);

    let mut push_refspecs = vec![];

    for (pr, _) in deps {
        println!("\nWorking on PR: {:?}", pr.head());

        let from = rev_to_commit(&repo, pr.head());

        let mut walk = repo.revwalk().unwrap();
        walk.set_sorting(Sort::TOPOLOGICAL).unwrap();
        walk.set_sorting(Sort::REVERSE).unwrap();
        walk.push(from.id()).unwrap();
        walk.hide(stop_cherry_pick_at).unwrap();

        // TODO: Simplify by using rebase instead of cherry-pick
        // TODO: Skip if remote/<branch> is the same SHA as <branch> (only until the first cherry-pick)
        cherry_pick_range(&repo, &mut walk);

        // Record the commit (in the new stack) that the local branch should now point to.
        // Actually perform the switch later on in a batch so we don't leave the repo in
        // a troubled state if this process is interrupted.
        update_local_branches_to.push((pr.head(), head_commit(&repo)));

        // Use remote branch as boundary for the next cherry-pick
        let from = rev_to_commit(&repo, &remote_ref(remote, pr.head()));
        stop_cherry_pick_at = from.id();

        push_refspecs.push(format!("{}:refs/heads/{}", head_commit(&repo).id(), pr.head()));
    }

    let repo_dir = repo.workdir().unwrap().to_str().unwrap();

    // `libgit2` doesn't support refspecs containing raw SHAs, so we shell out
    // to `git push` instead. https://github.com/libgit2/libgit2/issues/1125
    let mut command = Command::new("git");
    command.arg("push").arg("-f").arg(remote);
    command.args(push_refspecs.as_slice());
    command.current_dir(repo_dir);

    println!("\n{:?}", push_refspecs);
    loop_until_confirm("Going to push these refspecs ☝️ ");

    command.spawn()?.await?;

    println!("\nUpdating local branches so they point to the new stack.\n");
    for (branch, target) in update_local_branches_to {
        println!("  + Branch {} now points to {}", branch, target.id());
        repo.branch(branch, &target, true).unwrap();
    }

    Ok(())
}
