use petgraph::visit::Bfs;
use petgraph::visit::EdgeRef;
use petgraph::{Direction, Graph};
use regex::Regex;
use std::rc::Rc;

use crate::api::search::PullRequest;

fn process(row: String) -> String {
    // TODO: Make this configurable
    let regex = Regex::new(r"\[HEAP-\d+\]\s*").unwrap();
    regex.replace_all(&row, "").into_owned()
}

pub fn build_table(graph: Graph<Rc<PullRequest>, usize>, title: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!("### Stacked PR Chain: {}\n", title));
    out.push_str("| PR | Title |  Merges Into  |\n");
    out.push_str("|:--:|:------|:-------------:|\n");

    // TODO: Use graph::log to simplify this
    let roots: Vec<_> = graph.externals(Direction::Incoming).collect();

    for root in roots {
        let mut bfs = Bfs::new(&graph, root);
        while let Some(node) = bfs.next(&graph) {
            let parent = graph.edges_directed(node, Direction::Incoming).next();
            let node: Rc<PullRequest> = graph[node].clone();

            let row = match parent {
                Some(parent) => format!(
                    "|#{}|{}|#{}|\n",
                    node.number(),
                    node.title(),
                    graph[parent.source().clone()].number()
                ),
                None => format!("|#{}|{}|**N/A**|\n", node.number(), node.title()),
            };

            out.push_str(&process(row));
        }
    }

    out
}
