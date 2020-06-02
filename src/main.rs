use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    println!("Hello, world!");
    Ok(())
    /*
    # TODO
    - [ ] Authentication (personal access token)
    - [ ] Fetch all PRs matching Jira
    - [ ] Construct graph
    - [ ] Create markdown table
    - [ ] Persist table back to Github
    */
}
