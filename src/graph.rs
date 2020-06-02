use std::collections::{HashMap, HashSet};
use petgraph::visit::IntoNodeReferences; 
use std::rc::Rc;
use std::cell::RefCell;
use petgraph::Graph;

use crate::api::search::PullRequest;

pub fn build(prs: Vec<Rc<PullRequest>>) -> Graph<Rc<PullRequest>, usize> {
    let mut tree = Graph::<Rc<PullRequest>, usize>::new();
    let heads = prs.iter().map(|pr| pr.head());
    let handles: Vec<_> = prs.iter().map(|pr| tree.add_node(pr.clone())).collect();
    let handles_by_head: HashMap<_, _> = heads.zip(handles.iter()).collect();

    for (i, pr) in prs.iter().enumerate() {
        let head_handle = handles[i];
        if let Some(&base_handle) = handles_by_head.get(pr.base()) {
            tree.add_edge(head_handle, *base_handle, 1);
        }
    }

    tree
}
