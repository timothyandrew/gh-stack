use clap::{App, AppSettings, Arg, ArgMatches, SubCommand};
use console::style;
use git2::Repository;
use std::env;
use std::error::Error;
use std::rc::Rc;

use gh_stack::api::PullRequest;
use gh_stack::graph::FlatDep;
use gh_stack::util::loop_until_confirm;
use gh_stack::Credentials;
use gh_stack::{api, git, graph, markdown, persist};

fn clap<'a, 'b>() -> App<'a, 'b> {
    let identifier = Arg::with_name("identifier")
        .index(1)
        .required(true)
        .help("All pull requests containing this identifier in their title form a stack");

    let exclude = Arg::with_name("exclude")
        .long("excl")
        .short("e")
        .multiple(true)
        .takes_value(true)
        .help("Exclude an issue from consideration (by number). Pass multiple times");

    let annotate = SubCommand::with_name("annotate")
        .about("Annotate the descriptions of all PRs in a stack with metadata about all PRs in the stack")
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(identifier.clone())
        .arg(exclude.clone())
        .arg(Arg::with_name("prelude")
                .long("prelude")
                .short("p")
                .value_name("FILE")
                .help("Prepend the annotation with the contents of this file"));

    let log = SubCommand::with_name("log")
        .about("Print a list of all pull requests in a stack to STDOUT")
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(exclude.clone())
        .arg(identifier.clone());

    let autorebase = SubCommand::with_name("autorebase")
        .about("Rebuild a stack based on changes to local branches and mirror these changes up to the remote")
        .arg(Arg::with_name("remote")
                .long("remote")
                .short("r")
                .value_name("REMOTE")
                .help("Name of the remote to (force-)push the updated stack to (default: `origin`)"))
        .arg(Arg::with_name("repo")
                .long("repo")
                .short("C")
                .value_name("PATH_TO_REPO")
                .help("Path to a local copy of the repository"))
        .arg(Arg::with_name("boundary")
                .long("initial-cherry-pick-boundary")
                .short("b")
                .value_name("SHA")
                .help("Stop the initial cherry-pick at this SHA (exclusive)"))
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(exclude.clone())
        .arg(identifier.clone());

    let rebase = SubCommand::with_name("rebase")
        .about(
            "Print a bash script to STDOUT that can rebase/update the stack (with a little help)",
        )
        .setting(AppSettings::ArgRequiredElseHelp)
        .arg(exclude.clone())
        .arg(identifier.clone());

    let app = App::new("gh-stack")
        .setting(AppSettings::SubcommandRequiredElseHelp)
        .setting(AppSettings::DisableVersion)
        .setting(AppSettings::VersionlessSubcommands)
        .setting(AppSettings::DisableHelpSubcommand)
        .subcommand(annotate)
        .subcommand(log)
        .subcommand(rebase)
        .subcommand(autorebase);

    app
}

async fn build_pr_stack(
    pattern: &str,
    credentials: &Credentials,
    exclude: Vec<String>,
) -> Result<FlatDep, Box<dyn Error>> {
    let prs = api::search::fetch_pull_requests_matching(pattern, &credentials).await?;

    let prs = prs
        .into_iter()
        .filter(|pr| !exclude.contains(&pr.number().to_string()))
        .map(Rc::new)
        .collect::<Vec<Rc<PullRequest>>>();
    let graph = graph::build(&prs);
    let stack = graph::log(&graph);
    Ok(stack)
}

fn get_excluded(m: &ArgMatches) -> Vec<String> {
    let excluded = m.values_of("exclude");

    match excluded {
        Some(excluded) => excluded.map(String::from).collect(),
        None => vec![],
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    dotenv::from_filename(".gh-stack").ok();

    let token = env::var("GHSTACK_OAUTH_TOKEN").expect("You didn't pass `GHSTACK_OAUTH_TOKEN`");
    let credentials = Credentials::new(&token);
    let matches = clap().get_matches();

    match matches.subcommand() {
        ("annotate", Some(m)) => {
            let identifier = m.value_of("identifier").unwrap();
            let stack = build_pr_stack(identifier, &credentials, get_excluded(m)).await?;
            let table = markdown::build_table(&stack, identifier, m.value_of("prelude"));

            for (pr, _) in stack.iter() {
                println!("{}: {}", pr.number(), pr.title());
            }
            loop_until_confirm("Going to update these PRs ☝️ ");

            persist::persist(&stack, &table, &credentials).await?;

            println!("Done!");
        }

        ("log", Some(m)) => {
            let identifier = m.value_of("identifier").unwrap();
            let stack = build_pr_stack(identifier, &credentials, get_excluded(m)).await?;

            for (pr, maybe_parent) in stack {
                match maybe_parent {
                    Some(parent) => {
                        let into = style(format!("(Merges into #{})", parent.number())).green();
                        println!("#{}: {} {}", pr.number(), pr.title(), into);
                    }

                    None => {
                        let into = style("(Base)").red();
                        println!("#{}: {} {}", pr.number(), pr.title(), into);
                    }
                }
            }
        }

        ("rebase", Some(m)) => {
            let identifier = m.value_of("identifier").unwrap();
            let stack = build_pr_stack(identifier, &credentials, get_excluded(m)).await?;

            let script = git::generate_rebase_script(stack);
            println!("{}", script);
        }

        ("autorebase", Some(m)) => {
            let identifier = m.value_of("identifier").unwrap();
            let stack = build_pr_stack(identifier, &credentials, get_excluded(m)).await?;

            let repo = m
                .value_of("repo")
                .expect("The --repo argument is required.");
            let repo = Repository::open(repo)?;

            let remote = m.value_of("remote").unwrap_or("origin");
            let remote = repo.find_remote(remote).unwrap();

            git::perform_rebase(stack, &repo, remote.name().unwrap(), m.value_of("boundary"))
                .await?;
            println!("All done!");
        }

        (_, _) => panic!("Invalid subcommand."),
    }

    Ok(())
    /*
    # TODO
    - [x] Authentication (personal access token)
    - [x] Fetch all PRs matching Jira
    - [x] Construct graph
    - [x] Create markdown table
    - [x] Persist table back to Github
    - [x] Accept a prelude via STDIN
    - [x] Log a textual representation of the graph
    - [x] Automate rebase
    - [x] Better CLI args
    - [ ] Build status icons
    - [ ] Panic on non-200s
    */
}
