use std::error::Error;
use std::env;
use std::collections::HashMap;
use std::process;

use gh_stack::api;
use gh_stack::Credentials;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let env: HashMap<String, String> = env::vars().collect();
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("usage: gh-stack <pattern>");
        process::exit(1);
    }

    let pattern = args.last().unwrap();
    let token = env.get("GHSTACK_OAUTH_TOKEN").expect("You didn't pass `GHSTACK_OAUTH_TOKEN`");

    let credentials = Credentials::new(token);
    api::search::fetch_pull_requests_matching(&pattern, &credentials).await?;

    Ok(())
    /*
    # TODO
    - [x] Authentication (personal access token)
    - [x] Fetch all PRs matching Jira
    - [ ] Construct graph
    - [ ] Create markdown table
    - [ ] Persist table back to Github
    */
}
