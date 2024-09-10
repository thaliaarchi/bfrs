use std::{
    fmt::{self, Debug, Formatter},
    hash::BuildHasher,
    ops::{Index, IndexMut},
};

use hashbrown::{hash_map::DefaultHashBuilder, HashTable};

/// A node in a graph.
#[derive(Clone, PartialEq, Eq, Hash)]
pub enum Node {
    /// Copy the value from the cell at the offset.
    Copy(isize),
    /// A constant value.
    Const(u8),
    /// A value read from the user.
    Input { id: usize },
    /// Addition of two values.
    Add(NodeId, NodeId),
    /// Multiplication of two values.
    Mul(NodeId, NodeId),
}

/// The ID of a node in a graph.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

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
}

impl Graph {
    /// Constructs an empty graph.
    #[inline]
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            table: HashTable::new(),
            hash_builder: DefaultHashBuilder::default(),
        }
    }

    /// Gets or inserts a node and returns its ID.
    pub fn insert(&mut self, node: Node) -> NodeId {
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
        let entry = self.table.entry(hash, eq, hasher).or_insert_with(|| {
            let Ok(index) = u32::try_from(self.nodes.len()) else {
                panic!("graph too large for u32 index");
            };
            self.nodes.push(node);
            NodeId(index)
        });
        *entry.get()
    }

    /// Gets the ID of a node.
    pub fn get(&self, node: &Node) -> Option<NodeId> {
        debug_assert!(self.assert_node(&node));
        let hash = self.hash_builder.hash_one(&node);
        let eq = |id: &NodeId| {
            let key = unsafe { self.nodes.get_unchecked(id.as_usize()) };
            node == key
        };
        Some(*self.table.find(hash, eq)?)
    }

    fn assert_id(&self, id: NodeId) -> bool {
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

impl Node {
    /// Gets or inserts this node into a graph and returns its ID.
    #[inline]
    pub fn insert(self, g: &mut Graph) -> NodeId {
        g.insert(self)
    }

    /// Gets the ID of this node in a graph.
    #[inline]
    pub fn get(&self, g: &Graph) -> Option<NodeId> {
        g.get(self)
    }
}

impl NodeId {
    /// Returns the index of this node ID.
    #[inline]
    pub fn as_usize(&self) -> usize {
        self.0 as usize
    }
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Graph ")?;
        f.debug_map()
            .entries(
                self.nodes
                    .iter()
                    .enumerate()
                    .map(|(i, node)| (NodeId(i as u32), node)),
            )
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
        write!(f, "%{}", self.0)
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
        let id1b = g.insert(Node::Const(1));
        let id2b = g.insert(Node::Add(id0b, id1b));
        assert_eq!(id0, id0b);
        assert_eq!(id1, id1b);
        assert_eq!(id2, id2b);
    }
}
