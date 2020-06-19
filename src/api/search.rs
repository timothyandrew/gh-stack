use futures::future::join_all;
use serde::Deserialize;
use std::error::Error;

use crate::{api, Credentials};

#[derive(Deserialize, Debug, Clone)]
pub struct SearchItem {
    url: String,
    title: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PullRequestRef {
    label: String,
    #[serde(rename = "ref")]
    gitref: String,
    sha: String,
}

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum PullRequestStatus {
    #[serde(rename = "open")]
    Open,
    #[serde(rename = "closed")]
    Closed,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PullRequest {
    id: usize,
    number: usize,
    head: PullRequestRef,
    base: PullRequestRef,
    title: String,
    url: String,
    body: String,
    state: PullRequestStatus,
}

impl PullRequest {
    pub fn head(&self) -> &str {
        &self.head.gitref
    }

    pub fn base(&self) -> &str {
        &self.base.gitref
    }

    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn number(&self) -> usize {
        self.number
    }

    pub fn title(&self) -> String {
        match &self.state {
            PullRequestStatus::Open => self.title.to_owned(),
            PullRequestStatus::Closed => format!("~~{}~~", &self.title.trim()),
        }
    }

    pub fn state(&self) -> &PullRequestStatus {
        &self.state
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub fn note(&self) -> &str {
        match &self.state {
            PullRequestStatus::Open => "N/A",
            PullRequestStatus::Closed => "Merged",
        }
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
