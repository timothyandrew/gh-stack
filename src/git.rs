use crate::api::search::PullRequestStatus;
use crate::graph::FlatDep;
use std::error::Error;
use git2::{Cred, ObjectType, Repository, Index, Sort, CherrypickOptions, Remote, Commit, PushOptions, RemoteCallbacks};
use git2::build::CheckoutBuilder;
use tokio::process::Command;
use dialoguer::Input;
use std::env;

fn remote_ref(remote: &str, git_ref: &str) -> String {
    format!("{}/{}", remote, git_ref)
}

fn loop_until_confirm(prompt: &str) {
    let prompt = format!("{} Type 'yes' to continue", prompt);
    loop {
        let result = Input::<String>::new().with_prompt(&prompt).interact().unwrap();
        match &result[..] {
            "yes" => return,
            _ => continue
        }
    }
}

/// or all open pull requests in the graph, generate a series of commands
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
        out.push_str(&format!("export FROM=\"{}\"\n\n", remote_ref("heap", from.head())));

        out.push_str("git checkout \"$TO\"\n");
        out.push_str("git cherry-pick \"$PREBASE\"..\"$FROM\"\n");
        out.push_str("export PREBASE=\"$(git rev-parse --verify $FROM)\"\n");
        out.push_str("git push -f heap HEAD:refs/heads/\"$FROM\"\n");
    }

    out
}

pub async fn perform_rebase(deps: FlatDep, repo: &Repository, remote: &str) -> Result<(), Box<dyn Error>> {
    let deps = deps
        .iter()
        .filter(|(dep, _)| *dep.state() == PullRequestStatus::Open)
        .collect::<Vec<_>>();
    
    let (pr, _) = deps[0];

    let base = remote_ref(remote, pr.base());
    let base = repo.revparse_single(&base).unwrap();
    let base = base.as_commit().unwrap();

    let head = pr.head();
    let head = repo.revparse_single(&head).unwrap();
    let head = head.as_commit().unwrap();

    let mut stop_cherry_pick_at = repo.merge_base(base.id(), head.id()).unwrap();

    println!("Checking out {:?}", base);
    repo.checkout_tree(&base.as_object(), None).unwrap();
    repo.set_head_detached(base.id()).unwrap();

    let mut push_refspecs = vec![];

    for (pr, _) in deps {
        println!("Working on PR: {:?}", pr.head());


        let from = repo.revparse_single(&pr.head()).unwrap();
        let from = from.as_commit().unwrap();

        let mut walk = repo.revwalk().unwrap();
        walk.set_sorting(Sort::TOPOLOGICAL).unwrap();
        walk.set_sorting(Sort::REVERSE).unwrap();
        walk.push(from.id()).unwrap();
        walk.hide(stop_cherry_pick_at).unwrap();

        // TODO: Simplify by using rebase instead of cherry-pick
        // TODO: Skip if remote/<branch> is the same SHA as <branch>
        for from in walk {
            let from = repo.find_commit(from.unwrap()).unwrap();
            let to = repo.find_commit(repo.head().unwrap().target().unwrap()).unwrap();

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
                index = repo.index().unwrap();
                index.read(true).unwrap();
            } 

            let tree = index.write_tree_to(&repo).unwrap();
            let tree = repo.find_tree(tree).unwrap();

            let signature = repo.signature().unwrap();
            let commit = repo.commit(None, &signature, &signature, &from.message().unwrap(), &tree, &[&to]).unwrap();
            let commit = repo.find_commit(commit).unwrap();

            let mut cb = CheckoutBuilder::new();
            cb.force();
            repo.checkout_tree(&commit.as_object(), Some(&mut cb)).unwrap();
            repo.set_head_detached(commit.id()).unwrap();

            // "Complete" the cherry-pick. There is likely a better way to do 
            // this that I haven't found so far.
            repo.cleanup_state().unwrap();
        }

        // Update local branch
        let head = repo.head().unwrap().target().unwrap();
        let head = repo.find_commit(head).unwrap();
        repo.branch(pr.head(), &head, true).unwrap();

        // Use remote branch as boundary for next cherry-pick
        let from = repo.revparse_single(&remote_ref(remote, pr.head())).unwrap();
        let from = from.as_commit().unwrap();
        stop_cherry_pick_at = from.id();

        push_refspecs.push(format!("refs/heads/{}:refs/heads/{}", pr.head(), pr.head()));
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

    Ok(())
}