use serde::Serialize;
use serde::Deserialize;
use std::error::Error;
use std::rc::Rc;

use crate::{api, Credentials};

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
    draft: bool
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
        let title = self.title.trim();
        let title = match &self.draft {
            true => format!("*(Draft) {}*", title),
            false => title.to_owned()
        };

        match &self.state {
            PullRequestStatus::Open => title,
            PullRequestStatus::Closed => format!("~~{}~~", title),
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


#[derive(Serialize, Debug)]
struct UpdateDescriptionRequest<'a> {
    body: &'a str,
}

pub async fn update_description(
    description: String,
    pr: Rc<PullRequest>,
    c: &Credentials,
) -> Result<(), Box<dyn Error>> {
    let client = reqwest::Client::new();
    let body = UpdateDescriptionRequest { body: &description };
    let request = api::base_patch_request(&client, &c, pr.url()).json(&body);
    request.send().await?;
    Ok(())
}
