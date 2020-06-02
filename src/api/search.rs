use futures::future::join_all;
use serde::Deserialize;
use std::error::Error;

use crate::{api, Credentials};

#[derive(Deserialize, Debug)]
pub struct SearchItem {
    url: String,
    title: String,
}

#[derive(Deserialize, Debug)]
pub struct PullRequestRef {
    label: String,
    r#ref: String,
    sha: String,
}

#[derive(Deserialize, Debug)]
pub struct PullRequest {
    id: usize,
    head: PullRequestRef,
    base: PullRequestRef,
    title: String,
}

impl PullRequest {
    pub fn head(&self) -> &str {
        &self.head.label
    }

    pub fn base(&self) -> &str {
        &self.base.label
    }
}

#[derive(Deserialize, Debug)]
struct SearchResponse {
    items: Vec<SearchItem>,
}

pub async fn fetch_pull_requests_matching(
    pattern: &str,
    credentials: &Credentials,
) -> Result<Vec<PullRequest>, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let request = api::base_request(
        &client,
        &credentials,
        "https://api.github.com/search/issues",
    )
    .query(&[("q", format!("{} in:title", pattern))]);

    let items = request.send().await?.json::<SearchResponse>().await?.items;

    let item_futures = items.into_iter().map(|item| {
        api::base_request(&client, &credentials, &item.url.replace("issues", "pulls")).send()
    });

    // The `unwrap`s are required here because both `reqwest::send` and `reqwest::json` return a `Result` which has
    // to be unwrapped after the future has been `await`ed on.
    let items = join_all(item_futures)
        .await
        .into_iter()
        .map(|item| item.unwrap());
    let responses: Vec<_> = join_all(items.map(|item| item.json::<PullRequest>()))
        .await
        .into_iter()
        .map(|item| item.unwrap())
        .collect();

    Ok(responses)
}
