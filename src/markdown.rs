use crate::api::search::PullRequest;
use std::fmt::Display;

pub trait AsMarkdown {
    fn as_markdown_table_row(&self) -> String;
}

pub fn build_table<T: Display>(graph: &[T]) {}
