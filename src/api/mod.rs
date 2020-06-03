use crate::Credentials;
use reqwest::{Client, RequestBuilder};
use std::time::Duration;

pub mod pull_request;
pub mod search;

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
