use futures::future::join_all;
use serde::Deserialize;
use std::error::Error;

use crate::{api, markdown, Credentials};

#[derive(Deserialize, Debug, Clone)]
pub struct SearchItem {
    url: String,
    title: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PullRequestRef {
    label: String,
    r#ref: String,
    sha: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PullRequest {
    id: usize,
    number: usize,
    head: PullRequestRef,
    base: PullRequestRef,
    merges_into: Option<Box<PullRequest>>,
    title: String,
}

impl PullRequest {
    pub fn head(&self) -> &str {
        &self.head.label
    }

    pub fn base(&self) -> &str {
        &self.base.label
    }

    pub fn set_merges_into(&mut self, into: PullRequest) {
        // `clone` here to avoid an explosion of lifetime specifiers
        self.merges_into = Some(Box::new(into))
    }
}

// impl markdown::AsMarkdown for PullRequest {
//     fn as_markdown_table_row(&self) -> String {
//         match self.merges_into {
//             Some(into) => format!("|#{}|{}|#{}|", self.number, self.title, into.number),
//             None => format!("|#{}|{}|`develop`/feature branch|", self.number, self.title),
//         }
//     }
// }

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
