use futures::future::join_all;
use serde::Deserialize;
use std::error::Error;

use crate::api::{PullRequest, PullRequestReview};
use crate::{api, Credentials};

#[derive(Deserialize, Debug, Clone)]
pub struct SearchItem {
    url: String,
    title: String,
}

#[derive(Deserialize, Debug)]
struct SearchResponse {
    items: Vec<SearchItem>,
}

pub async fn fetch_reviews_for_pull_request(
    pr: &PullRequest,
    credentials: &Credentials,
) -> Result<Vec<PullRequestReview>, Box<dyn Error>> {
    let client = reqwest::Client::new();

    let request = api::base_request(&client, &credentials, &format!("{}/reviews", pr.url())[..]);

    let reviews = request
        .send()
        .await?
        .json::<Vec<PullRequestReview>>()
        .await?;

    Ok(reviews)
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
    .query(&[("q", format!("\"{}\" in:title", pattern))]);

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
        .map(|item| async {
            let pr = item.unwrap();
            let pr = pr.fetch_reviews(credentials).await.unwrap();
            pr
        })
        .collect();

    Ok(join_all(responses).await)
}
