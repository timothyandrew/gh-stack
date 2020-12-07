use serde::Deserialize;
use serde::Serialize;
use std::error::Error;
use std::rc::Rc;

use crate::api::search;
use crate::{api, Credentials};

#[derive(Deserialize, Debug, Clone, PartialEq)]
#[allow(non_camel_case_types)]
pub enum PullRequestReviewState {
    APPROVED,
    PENDING,
    CHANGES_REQUESTED,
    DISMISSED,
    COMMENTED,
}

#[derive(Deserialize, Debug, Clone)]
pub struct PullRequestReview {
    state: PullRequestReviewState,
    body: String,
}

impl PullRequestReview {
    pub fn is_approved(&self) -> bool {
        self.state == PullRequestReviewState::APPROVED
    }
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
    draft: bool,
    #[serde(skip)]
    reviews: Vec<PullRequestReview>,
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
            false => title.to_owned(),
        };

        match &self.state {
            PullRequestStatus::Open => title,
            PullRequestStatus::Closed => format!("~~{}~~", title),
        }
    }

    pub fn state(&self) -> &PullRequestStatus {
        &self.state
    }

    pub fn review_state(&self) -> PullRequestReviewState {
        if self.at_least_one_approval() {
            PullRequestReviewState::APPROVED
        } else {
            PullRequestReviewState::PENDING
        }
    }

    pub fn body(&self) -> &str {
        &self.body
    }

    pub async fn fetch_reviews(
        self,
        credentials: &Credentials,
    ) -> Result<PullRequest, Box<dyn Error>> {
        let reviews = search::fetch_reviews_for_pull_request(&self, credentials).await?;

        let pr = PullRequest { reviews, ..self };

        Ok(pr)
    }

    fn at_least_one_approval(&self) -> bool {
        self.reviews.iter().any(|review| review.is_approved())
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
