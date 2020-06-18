use futures::future::join_all;
use regex::Regex;
use std::error::Error;

use crate::api::pull_request;
use crate::graph::FlatDep;
use crate::Credentials;

const SHIELD_OPEN: &str = "<!---GHSTACKOPEN-->";
const SHIELD_CLOSE: &str = "<!---GHSTACKCLOSE-->";

fn safe_replace(body: &str, table: &str) -> String {
    let new = format!("\n{}\n{}\n{}\n", SHIELD_OPEN, table, SHIELD_CLOSE);

    if body.contains(SHIELD_OPEN) {
        let matcher = format!(
            "(?s){}.*{}",
            regex::escape(SHIELD_OPEN),
            regex::escape(SHIELD_CLOSE)
        );
        let re = Regex::new(&matcher).unwrap();
        re.replace_all(body, &new[..]).into_owned()
    } else {
        let mut body: String = body.to_owned();
        body.push_str(&new);
        body
    }
}

pub async fn persist(
    prs: &FlatDep,
    table: &str,
    c: &Credentials,
) -> Result<(), Box<dyn Error>> {
    let futures = prs.iter().map(|(pr, _)| {
        let description = safe_replace(pr.body(), table);
        pull_request::update_description(description, pr.clone(), c)
    });

    let results = join_all(futures.collect::<Vec<_>>()).await;

    for result in results {
        result.unwrap();
    }

    Ok(())
}
