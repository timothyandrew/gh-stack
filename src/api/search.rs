use std::error::Error;

use crate::Credentials;

pub async fn fetch_pull_requests_matching(
  pattern: &str,
  credentials: &Credentials,
) -> Result<(), Box<dyn Error>> {
  let client = reqwest::Client::new();
  let request = client
      .get("https://api.github.com/search/issues")
      .query(&[("q", format!("{} in:title", pattern))])
      .header("Authorization", format!("token {}", credentials.token))
      .header("User-Agent", "timothyandrew/gh-stack");
  let response = request.send().await?.text().await?;
  println!("{}", response);
  Ok(())
}