use crate::Credentials;
use reqwest::{Client, RequestBuilder};
use std::time::Duration;

pub mod pull_request;
pub mod search;

pub use pull_request::PullRequest;
pub use pull_request::PullRequestReview;
pub use pull_request::PullRequestReviewState;
pub use pull_request::PullRequestStatus;

fn base_request(client: &Client, credentials: &Credentials, url: &str) -> RequestBuilder {
    client
        .get(url)
        .timeout(Duration::from_secs(5))
        .header("Authorization", format!("token {}", credentials.token))
        .header("User-Agent", "timothyandrew/gh-stack")
}

fn base_patch_request(client: &Client, credentials: &Credentials, url: &str) -> RequestBuilder {
    client
        .patch(url)
        .timeout(Duration::from_secs(5))
        .header("Authorization", format!("token {}", credentials.token))
        .header("User-Agent", "timothyandrew/gh-stack")
}
