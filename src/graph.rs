#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    fmt::{self, Debug, Formatter},
    hash::{BuildHasher, Hash, Hasher},
    ops::{Deref, Index, IndexMut},
    ptr,
};

use hashbrown::{hash_map::DefaultHashBuilder, HashTable};

use crate::node::Node;

/// A graph of unique nodes, structured as an arena.
///
/// # Safety
///
/// It is undefined behavior to use a `NodeId` in any graph other than the one
/// which created it.
#[derive(Clone)]
pub struct Graph {
    nodes: Vec<Node>,
    table: HashTable<NodeId>,
    hash_builder: DefaultHashBuilder,
    #[cfg(debug_assertions)]
    graph_id: u32,
}

/// The ID of a node in a graph.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    index: u32,
    #[cfg(debug_assertions)]
    graph_id: u32,
}

/// A reference to a node in a graph.
#[derive(Clone, Copy)]
pub struct NodeRef<'g> {
    graph: &'g Graph,
    index: u32,
}

#[cfg(debug_assertions)]
static GRAPH_ID: AtomicU32 = AtomicU32::new(0);

macro_rules! insert(($self:expr, $node:expr) => {{
    let Ok(index) = u32::try_from($self.nodes.len()) else {
        panic!("graph too large for u32 index");
    };
    $self.nodes.push($node);
    NodeId {
        index,
        #[cfg(debug_assertions)]
        graph_id: $self.graph_id,
    }
}});

impl Graph {
    /// Constructs an empty graph.
    #[inline]
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            table: HashTable::new(),
            hash_builder: DefaultHashBuilder::default(),
            #[cfg(debug_assertions)]
            graph_id: GRAPH_ID.fetch_add(1, Ordering::Relaxed),
        }
    }

    /// Inserts a node and returns its ID. [`Node::Copy`] and [`Node::Input`]
    /// are not deduplicated.
    pub fn insert(&mut self, node: Node) -> NodeId {
        match &node {
            Node::Copy(_) | Node::Input { .. } => insert!(self, node),
            Node::Const(_) | Node::Add(_, _) | Node::Mul(_, _) => self.get_or_insert(node),
        }
    }

    /// Gets or inserts a node and returns its ID.
    fn get_or_insert(&mut self, node: Node) -> NodeId {
        debug_assert!(self.assert_node(&node));
        let hash = self.hash_builder.hash_one(&node);
        let eq = |id: &NodeId| {
            let key = unsafe { self.nodes.get_unchecked(id.as_usize()) };
            &node == key
        };
        let hasher = |id: &NodeId| {
            let key = unsafe { self.nodes.get_unchecked(id.as_usize()) };
            self.hash_builder.hash_one(key)
        };
        *self
            .table
            .entry(hash, eq, hasher)
            .or_insert_with(|| insert!(self, node))
            .get()
    }

    /// Gets the ID of a node.
    pub fn find(&self, node: &Node) -> Option<NodeId> {
        debug_assert!(self.assert_node(&node));
        let hash = self.hash_builder.hash_one(&node);
        let eq = |id: &NodeId| {
            let key = unsafe { self.nodes.get_unchecked(id.as_usize()) };
            node == key
        };
        Some(*self.table.find(hash, eq)?)
    }

    /// Gets a reference to the identified node.
    pub fn get(&self, id: NodeId) -> NodeRef<'_> {
        debug_assert!(self.assert_id(id));
        NodeRef {
            graph: self,
            index: id.index,
        }
    }

    fn assert_id(&self, id: NodeId) -> bool {
        #[cfg(debug_assertions)]
        if id.graph_id != self.graph_id {
            return false;
        }
        (id.as_usize()) < self.nodes.len()
    }

    fn assert_node(&self, node: &Node) -> bool {
        match *node {
            Node::Copy(_) | Node::Const(_) | Node::Input { .. } => true,
            Node::Add(lhs, rhs) | Node::Mul(lhs, rhs) => self.assert_id(lhs) && self.assert_id(rhs),
        }
    }
}

impl Index<NodeId> for Graph {
    type Output = Node;

    #[inline]
    fn index(&self, id: NodeId) -> &Self::Output {
        debug_assert!(self.assert_id(id));
        unsafe { self.nodes.get_unchecked(id.as_usize()) }
    }
}

impl IndexMut<NodeId> for Graph {
    #[inline]
    fn index_mut(&mut self, id: NodeId) -> &mut Self::Output {
        debug_assert!(self.assert_id(id));
        unsafe { self.nodes.get_unchecked_mut(id.as_usize()) }
    }
}

impl Default for Graph {
    #[inline]
    fn default() -> Self {
        Graph::new()
    }
}

impl NodeId {
    /// Returns the index of this node ID.
    #[inline]
    pub fn as_usize(&self) -> usize {
        self.index as usize
    }

    /// Gets a reference to this node.
    pub fn get<'g>(&self, g: &'g Graph) -> NodeRef<'g> {
        g.get(*self)
    }
}

impl<'g> NodeRef<'g> {
    /// Returns the ID of this node.
    #[inline]
    pub fn id(&self) -> NodeId {
        NodeId {
            index: self.index,
            #[cfg(debug_assertions)]
            graph_id: self.graph.graph_id,
        }
    }

    /// Returns this node.
    #[inline]
    pub fn node(&self) -> &'g Node {
        &self.graph[self.id()]
    }

    /// Returns the graph that contains this node.
    #[inline]
    pub fn graph(&self) -> &'g Graph {
        self.graph
    }
}

impl Deref for NodeRef<'_> {
    type Target = Node;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.node()
    }
}

impl PartialEq for NodeRef<'_> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.graph, other.graph) && self.index == other.index
    }
}

impl Eq for NodeRef<'_> {}

impl Hash for NodeRef<'_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id().hash(state)
    }
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Graph ")?;
        f.debug_map()
            .entries(self.nodes.iter().enumerate().map(|(i, node)| {
                let id = NodeId {
                    index: i as u32,
                    #[cfg(debug_assertions)]
                    graph_id: self.graph_id,
                };
                (id, node)
            }))
            .finish()
    }
}

impl Debug for Node {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match *self {
            Node::Copy(offset) => write!(f, "copy {offset}"),
            Node::Const(c) => write!(f, "const {c}"),
            Node::Input { id } => write!(f, "input {id}"),
            Node::Add(lhs, rhs) => write!(f, "add {lhs:?} {rhs:?}"),
            Node::Mul(lhs, rhs) => write!(f, "mul {lhs:?} {rhs:?}"),
        }
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "%{}", self.index)
    }
}

impl Debug for NodeRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn group(f: &mut Formatter<'_>, node: NodeRef<'_>, grouped: bool) -> fmt::Result {
            if grouped {
                write!(f, "({node:?})")
            } else {
                write!(f, "{node:?}")
            }
        }
        let g = self.graph();
        match *self.node() {
            Node::Copy(offset) => write!(f, "@{offset}'{}", self.index),
            Node::Const(value) => write!(f, "{value}"),
            Node::Input { id } => write!(f, "in{id}'{}", self.index),
            Node::Add(lhs, rhs) => {
                write!(f, "{:?} + ", &g.get(lhs))?;
                group(f, g.get(rhs), matches!(g[rhs], Node::Add(..)))
            }
            Node::Mul(lhs, rhs) => {
                group(f, g.get(lhs), matches!(g[lhs], Node::Add(..)))?;
                write!(f, " * ")?;
                group(
                    f,
                    g.get(rhs),
                    matches!(g[rhs], Node::Add(..) | Node::Mul(..)),
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::graph::{Graph, Node};

    #[test]
    fn insert_unique() {
        let mut g = Graph::new();
        let id0 = g.insert(Node::Copy(0));
        let id1 = g.insert(Node::Const(1));
        let id2 = g.insert(Node::Add(id0, id1));
        let id0b = g.insert(Node::Copy(0));
        assert_ne!(id0, id0b);
        let id1b = g.insert(Node::Const(1));
        let id2b = g.insert(Node::Add(id0, id1b));
        assert_eq!(id1, id1b);
        assert_eq!(id2, id2b);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn compare_mixed_ids() {
        let mut g1 = Graph::new();
        let mut g2 = Graph::new();
        let id1 = g1.insert(Node::Const(1));
        let id2 = g2.insert(Node::Const(2));
        assert_eq!(id1.as_usize(), id2.as_usize());
        assert_ne!(id1, id2);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn insert_mixed_ids() {
        let mut g1 = Graph::new();
        let mut g2 = Graph::new();
        let id1 = g1.insert(Node::Const(1));
        let id2 = g1.insert(Node::Const(2));
        g2.insert(Node::Add(id1, id2));
    }
}
