pub mod api;
pub mod git;
pub mod graph;
pub mod markdown;
pub mod persist;
pub mod util;

pub struct Credentials {
    // Personal access token
    token: String,
}

impl Credentials {
    pub fn new(token: &str) -> Credentials {
        Credentials {
            token: token.to_string(),
        }
    }
}
