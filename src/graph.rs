use petgraph::visit::Bfs;
use petgraph::visit::EdgeRef;
use petgraph::{Direction, Graph};
use std::collections::HashMap;
use std::rc::Rc;

use crate::api::search::PullRequest;

pub type FlatDep = Vec<(Rc<PullRequest>, Option<Rc<PullRequest>>)>;

pub fn build(prs: &[Rc<PullRequest>]) -> Graph<Rc<PullRequest>, usize> {
    let mut tree = Graph::<Rc<PullRequest>, usize>::new();
    let heads = prs.iter().map(|pr| pr.head());
    let handles: Vec<_> = prs.iter().map(|pr| tree.add_node(pr.clone())).collect();
    let handles_by_head: HashMap<_, _> = heads.zip(handles.iter()).collect();

    for (i, pr) in prs.iter().enumerate() {
        let head_handle = handles[i];
        if let Some(&base_handle) = handles_by_head.get(pr.base()) {
            tree.add_edge(*base_handle, head_handle, 1);
        }
    }

    tree
}

/// Return a flattened list of graph nodes as tuples; each tuple is `(node, node's parent [if exists])`.
/// TODO: Panic if this isn't a single flat list of dependencies
pub fn log(graph: &Graph<Rc<PullRequest>, usize>) -> FlatDep {
    let roots: Vec<_> = graph.externals(Direction::Incoming).collect();
    let mut out = Vec::new();

    for root in roots {
        let mut bfs = Bfs::new(&graph, root);
        while let Some(node) = bfs.next(&graph) {
            let parent = graph.edges_directed(node, Direction::Incoming).next();
            let node: Rc<PullRequest> = graph[node].clone();

            match parent {
                Some(parent) => out.push((node, Some(graph[parent.source()].clone()))),
                None => out.push((node, None)),
            }
        }
    }

    out.sort_by_key(|(dep, _)| {
        dep.state().clone()
    });

    out
}
