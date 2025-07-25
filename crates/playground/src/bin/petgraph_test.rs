use std::{fs, iter::Peekable};

use anyhow::Result;
use petgraph::{
	dot::{Config, Dot},
	graph::{DiGraph, NodeIndex},
};

#[derive(Debug, Clone)]
pub enum Operation {
	ChangeValue(i8),
	ShiftPtr(i32),
	Input,
	Output,
	Loop(Vec<NodeIndex>),
}

const BF: &str = include_str!("../../../../programs/hello_world.bf");

fn main() -> Result<()> {
	let graph = parse(BF);

	let dot = Dot::with_config(dbg!(&graph), &[Config::EdgeNoLabel]);

	fs::write("../../out/playground.dot", format!("{dot:?}"))?;

	for node in graph.node_indices() {
		println!("{:?}", graph[node]);
	}

	Ok(())
}

fn parse(s: &str) -> DiGraph<Operation, ()> {
	let mut graph = DiGraph::new();
	let mut chars = s.chars().peekable();

	parse_inner(&mut chars, &mut graph);

	graph
}

fn parse_inner<I>(
	chars: &mut Peekable<I>,
	graph: &mut DiGraph<Operation, ()>,
) -> (Vec<NodeIndex>, Option<NodeIndex>)
where
	I: Iterator<Item = char>,
{
	let mut nodes = Vec::new();
	let mut prev = None;

	while let Some(&c) = chars.peek() {
		let node = match c {
            '+' => {
                chars.next();
                Some(graph.add_node(Operation::ChangeValue(1)))
            }
            '-' => {
                chars.next();
                Some(graph.add_node(Operation::ChangeValue(-1)))
            }
            '>' => {
                chars.next();
                Some(graph.add_node(Operation::ShiftPtr(1)))
            }
            '<' => {
                chars.next();
                Some(graph.add_node(Operation::ShiftPtr(-1)))
            }
			'.' => {
				chars.next();
				Some(graph.add_node(Operation::Output))
			}
			',' => {
				chars.next();
				Some(graph.add_node(Operation::Input))
			}
			'[' => {
				chars.next();
				let (body_nodes, _) = parse_inner(chars, graph);
				let loop_node = graph.add_node(Operation::Loop(body_nodes.clone()));
				for i in 0..body_nodes.len().saturating_sub(1) {
					graph.add_edge(body_nodes[i], body_nodes[i + 1], ());
				}
				Some(loop_node)
			}
			']' => {
				chars.next();
				break;
			}
			_ => {
				chars.next();
				None
			}
		};

		if let Some(node) = node {
			if let Some(prev_node) = prev {
				graph.add_edge(prev_node, node, ());
			}

			prev = Some(node);
			nodes.push(node);
		}
	}

	(nodes, prev)
}
