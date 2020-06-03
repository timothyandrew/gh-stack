use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::process;
use std::rc::Rc;

use gh_stack::Credentials;
use gh_stack::{api, graph, markdown, persist};

use std::io::{self, Write};

pub fn read_cli_input(message: &str) -> String {
    print!("{}", message);
    io::stdout().flush().unwrap();

    let mut buf = String::new();
    io::stdin().read_line(&mut buf).unwrap();

    buf.trim().to_owned()
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let env: HashMap<String, String> = env::vars().collect();
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("usage: gh-stack <pattern>");
        process::exit(1);
    }

    let pattern = args.last().unwrap();
    let token = env
        .get("GHSTACK_OAUTH_TOKEN")
        .expect("You didn't pass `GHSTACK_OAUTH_TOKEN`");

    let credentials = Credentials::new(token);

    let prs = api::search::fetch_pull_requests_matching(&pattern, &credentials).await?;
    let prs = prs.into_iter().map(|pr| Rc::new(pr)).collect();
    let tree = graph::build(&prs);
    let table = markdown::build_table(tree, pattern);

    for pr in prs.iter() {
        println!("{}: {}", pr.number(), pr.title());
    }

    let response = read_cli_input("Going to update these PRs ☝️ (y/n): ");
    match &response[..] {
        "y" => persist::persist(&prs, &table, &credentials).await?,
        _ => std::process::exit(1),
    }

    persist::persist(&prs, &table, &credentials).await?;

    println!("Done!");

    Ok(())
    /*
    # TODO
    - [x] Authentication (personal access token)
    - [x] Fetch all PRs matching Jira
    - [x] Construct graph
    - [x] Create markdown table
    - [ ] Persist table back to Github
    */
}
