use git2::Repository;
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::process;
use std::rc::Rc;

use gh_stack::api::search::PullRequest;
use gh_stack::Credentials;
use gh_stack::{api, git, graph, markdown, persist};

pub fn read_cli_input(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();

    buf.trim().to_owned()
}

fn build_final_output(prelude_path: &str, tail: &str) -> String {
    let prelude = fs::read_to_string(prelude_path).unwrap();
    let mut out = String::new();

    out.push_str(&prelude);
    out.push_str("\n");
    out.push_str(&tail);

    out
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let env: HashMap<String, String> = env::vars().collect();
    let args: Vec<String> = env::args().collect();

    if args.len() > 4 {
        println!("usage: gh-stack <command=save|log|rebase> <pattern> <prelude_filename?>");
        process::exit(1);
    }

    let command = &args[1][..];
    let pattern = &args[2];
    let prelude = args.get(3);

    let token = env
        .get("GHSTACK_OAUTH_TOKEN")
        .expect("You didn't pass `GHSTACK_OAUTH_TOKEN`");

    let credentials = Credentials::new(token);

    let prs = api::search::fetch_pull_requests_matching(&pattern, &credentials).await?;
    let prs = prs
        .into_iter()
        .map(Rc::new)
        .collect::<Vec<Rc<PullRequest>>>();
    let tree = graph::build(&prs);

    match command {
        "github" => {
            let table = markdown::build_table(graph::log(&tree), pattern);

            let output = match prelude {
                Some(prelude) => build_final_output(prelude, &table),
                None => table,
            };

            for pr in prs.iter() {
                println!("{}: {}", pr.number(), pr.title());
            }

            let response = read_cli_input("Going to update these PRs ☝️ (y/n): ");
            match &response[..] {
                "y" => persist::persist(&prs, &output, &credentials).await?,
                _ => std::process::exit(1),
            }

            println!("Done!");
        }

        "rebase" => {
            let deps = graph::log(&tree);
            let script = git::generate_rebase_script(deps);
            println!("{}", script);
        }

        "autorebase" => {
            let deps = graph::log(&tree);
            let repo = Repository::open(prelude.unwrap()).unwrap();
            // TODO: Make this configurable
            let remote = repo.find_remote("heap").unwrap();
            git::perform_rebase(deps, &repo, remote.name().unwrap()).await?;
            println!("All done!");
        }

        "log" => {
            let log = graph::log(&tree);
            for (pr, maybe_parent) in log {
                match maybe_parent {
                    Some(parent) => println!("{} → {}", pr.head(), parent.head()),
                    None => println!("{} → N/A", pr.head()),
                }
            }
        }

        _ => panic!("Invalid command!"),
    };

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
    - [ ] Build status icons
    - [ ] Panic on non-200s
    */
}
