use std::collections::{HashMap, HashSet};

use petgraph::{algo::toposort, dot::Dot, prelude::*, stable_graph::DefaultIx};

fn main() {}

#[derive(Debug, Clone)]
struct BbDag {
	pub graph: BbGraph,
	pub live: LiveMap,
	pub users: UserMap,
	current_users: UserMap,
	beginning: BbGraphNodeIndex,
	pub last_io: Option<BbGraphNodeIndex>,
}

impl DagProperties for BbDag {
	fn has_body(&self) -> bool {
		true
	}

	fn has_conditional(&self) -> bool {
		self.graph.node_weights().any(DagProperties::has_conditional)
	}

	fn has_loop(&self) -> bool {
		self.graph.node_weights().any(DagProperties::has_loop)
	}

	fn has_io(&self) -> bool {
		self.last_io.is_some()
	}

	fn read_offsets(&self) -> HashSet<TapeAddr> {
		let mut results = HashSet::new();
		for node in self.graph.node_weights() {
			results.extend(node.read_offsets());
		}

		results
	}

	fn write_offsets(&self) -> HashSet<TapeAddr> {
		let mut results = HashSet::new();
		for node in self.graph.node_weights() {
			results.extend(node.write_offsets());
		}

		results
	}
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
struct LinkData {
	pub id: BbGraphNodeIndex,
	pub cond: bool,
}

impl LinkData {
	const fn new(id: BbGraphNodeIndex, cond: bool) -> Self {
		Self { id, cond }
	}
}

struct NodeLinks {
	pub io_before: Option<BbGraphNodeIndex>,
	pub io_after: Option<BbGraphNodeIndex>,
	pub in_uses: HashMap<TapeAddr, LinkData>,
	pub in_replaces: HashMap<TapeAddr, LinkData>,
	pub in_clobbering: UserMap,
	pub out_uses: UserMap,
	pub out_replaces: HashMap<TapeAddr, LinkData>,
	pub out_clobbering: UserMap,
}

#[derive(Debug, Clone)]
struct BbDagNode {
	pub op: BbDagOperation,
	pub offset: TapeAddr,
}

impl BbDagNode {
	fn body(&self) -> Option<&BbDag> {
		self.op.body()
	}

	fn body_mut(&mut self) -> Option<&mut BbDag> {
		self.op.body_mut()
	}
}

impl DagProperties for BbDagNode {
	fn has_body(&self) -> bool {
		matches!(self.op, BbDagOperation::Loop(..) | BbDagOperation::If(..))
	}

	fn has_conditional(&self) -> bool {
		self.has_body()
	}

	fn has_loop(&self) -> bool {
		match &self.op {
			BbDagOperation::Loop(..) => true,
			BbDagOperation::If(body) => body.has_loop(),
			_ => false,
		}
	}

	fn has_io(&self) -> bool {
		match &self.op {
			BbDagOperation::If(body) | BbDagOperation::Loop(body) => body.has_io(),
			BbDagOperation::Input | BbDagOperation::Output => true,
			_ => false,
		}
	}

	fn read_offsets(&self) -> HashSet<TapeAddr> {
		HashSet::new()
	}

	fn write_offsets(&self) -> HashSet<TapeAddr> {
		HashSet::new()
	}
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BbDagEdge {
	Using { addr: TapeAddr, cond: bool },
	Replacing { addr: TapeAddr, cond: bool },
	Clobbering { addr: TapeAddr, cond: bool },
	Io,
}

#[derive(Debug, Clone)]
enum BbDagOperation {
	Start,
	Loop(Box<BbDag>),
	If(Box<BbDag>),
	Add(u8),
	Set(u8),
	Input,
	Output,
}

impl BbDagOperation {
	fn body(&self) -> Option<&BbDag> {
		match self {
			Self::Loop(body) | Self::If(body) => Some(body.as_ref()),
			_ => None,
		}
	}

	fn body_mut(&mut self) -> Option<&mut BbDag> {
		match self {
			Self::Loop(body) | Self::If(body) => Some(body.as_mut()),
			_ => None,
		}
	}
}

trait DagProperties {
	fn has_body(&self) -> bool;

	fn has_conditional(&self) -> bool;

	fn has_loop(&self) -> bool;

	fn has_io(&self) -> bool;

	fn read_offsets(&self) -> HashSet<TapeAddr>;

	fn write_offsets(&self) -> HashSet<TapeAddr>;
}

type BbGraphIx = DefaultIx;
type BbGraphNodeIndex = NodeIndex<BbGraphIx>;
type BbGraphEdgeIndex = EdgeIndex<BbGraphIx>;
type BbGraphNodeSet = HashSet<BbGraphNodeIndex>;
type LinkSet = HashSet<LinkData>;
type BbGraph = StableDiGraph<BbDagNode, BbDagEdge, BbGraphIx>;
type LiveMap = HashMap<i32, BbGraphNodeIndex>;
type UserMap = HashMap<i32, LinkSet>;
type TapeAddr = i32;
