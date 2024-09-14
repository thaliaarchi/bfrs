#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    fmt::{self, Debug, Formatter},
    hash::{BuildHasher, Hash, Hasher},
    ops::{Deref, Index, IndexMut},
    ptr,
};

use hashbrown::{hash_map::DefaultHashBuilder, HashTable};

use crate::{
    graph::{ArrayId, ByteId},
    node::{Array, Byte, Node},
};

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
pub struct NodeRef<'g, T> {
    graph: &'g Graph,
    id: T,
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
    pub fn insert_byte(&mut self, node: Byte) -> ByteId {
        self.assert_byte(&node);
        match &node {
            Byte::Copy(_) | Byte::Input { .. } => ByteId(insert!(self, Node::Byte(node))),
            Byte::Const(_) | Byte::Add(_, _) | Byte::Mul(_, _) => {
                ByteId(self.get_or_insert(Node::Byte(node)))
            }
        }
    }

    /// Inserts a node and returns its ID. [`Node::Copy`] and [`Node::Input`]
    /// are not deduplicated.
    pub fn insert_array(&mut self, node: Array) -> ArrayId {
        self.assert_array(&node);
        ArrayId(self.get_or_insert(Node::Array(node)))
    }

    /// Gets or inserts a node and returns its ID.
    fn get_or_insert(&mut self, node: Node) -> NodeId {
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
        self.assert_node(&node);
        let hash = self.hash_builder.hash_one(&node);
        let eq = |id: &NodeId| {
            let key = unsafe { self.nodes.get_unchecked(id.as_usize()) };
            node == key
        };
        Some(*self.table.find(hash, eq)?)
    }

    /// Gets a reference to the identified node.
    pub fn get<T>(&self, id: T) -> NodeRef<'_, T>
    where
        Self: Index<T>,
    {
        NodeRef { graph: self, id }
    }

    /// Returns the number of nodes in this graph.
    pub fn len(&self) -> usize {
        self.nodes.len()
    }
}

#[cfg(debug_assertions)]
impl Graph {
    fn assert_id(&self, id: NodeId) {
        assert!(
            id.graph_id == self.graph_id && (id.as_usize()) < self.nodes.len(),
            "graph accessed with ID from another graph",
        );
    }

    fn assert_byte_id(&self, id: ByteId) {
        self.assert_id(id.0);
        assert!(
            matches!(self[id.0], Node::Byte(_)),
            "node accessed with incorrect index type",
        );
    }

    fn assert_node(&self, node: &Node) {
        match node {
            Node::Byte(node) => self.assert_byte(node),
            Node::Array(node) => self.assert_array(node),
        }
    }

    fn assert_byte(&self, node: &Byte) {
        match node {
            Byte::Copy(_) | Byte::Const(_) | Byte::Input { .. } => {}
            &Byte::Add(lhs, rhs) | &Byte::Mul(lhs, rhs) => {
                self.assert_byte_id(lhs);
                self.assert_byte_id(rhs);
            }
        }
    }

    fn assert_array(&self, node: &Array) {
        for &id in &node.elements {
            self.assert_byte_id(id);
        }
    }
}

#[cfg(not(debug_assertions))]
impl Graph {
    fn assert_id(&self, _id: NodeId) {}
    fn assert_node(&self, _node: &Node) {}
    fn assert_byte(&self, _node: &Byte) {}
    fn assert_array(&self, _node: &Array) {}
}

impl Index<NodeId> for Graph {
    type Output = Node;

    #[inline]
    fn index(&self, id: NodeId) -> &Self::Output {
        self.assert_id(id);
        unsafe { self.nodes.get_unchecked(id.as_usize()) }
    }
}

impl IndexMut<NodeId> for Graph {
    #[inline]
    fn index_mut(&mut self, id: NodeId) -> &mut Self::Output {
        self.assert_id(id);
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
    pub fn get<'g>(&self, g: &'g Graph) -> NodeRef<'g, NodeId> {
        g.get(*self)
    }
}

impl<'g, T> NodeRef<'g, T>
where
    Graph: Index<T>,
    T: Copy,
{
    /// Returns the ID of this node.
    #[inline]
    pub fn id(&self) -> T {
        self.id
    }

    /// Returns this node.
    #[inline]
    pub fn node(&self) -> &'g <Graph as Index<T>>::Output {
        &self.graph[self.id]
    }

    /// Returns the graph that contains this node.
    #[inline]
    pub fn graph(&self) -> &'g Graph {
        self.graph
    }
}

impl<T> Deref for NodeRef<'_, T>
where
    Graph: Index<T>,
    T: Copy,
{
    type Target = <Graph as Index<T>>::Output;

    #[inline]
    fn deref(&self) -> &Self::Target {
        self.node()
    }
}

impl<T: PartialEq> PartialEq for NodeRef<'_, T> {
    fn eq(&self, other: &Self) -> bool {
        ptr::eq(self.graph, other.graph) && self.id == other.id
    }
}

impl<T: Eq> Eq for NodeRef<'_, T> {}

impl<T: Hash> Hash for NodeRef<'_, T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn write_node(f: &mut Formatter<'_>, i: usize, node: &Node) -> fmt::Result {
            write!(f, "%{i} : ")?;
            match node {
                Node::Byte(node) => write!(f, "byte = {node:?}"),
                Node::Array(node) => write!(f, "array = {node:?}"),
            }
        }
        write!(f, "Graph {{")?;
        if f.alternate() {
            if !self.nodes.is_empty() {
                write!(f, "\n")?;
            }
            for (i, node) in self.nodes.iter().enumerate() {
                write!(f, "    ")?;
                write_node(f, i, node)?;
                write!(f, "\n")?;
            }
        } else {
            for (i, node) in self.nodes.iter().enumerate() {
                if i != 0 {
                    write!(f, ";")?;
                }
                write!(f, " ")?;
                write_node(f, i, node)?;
            }
            if !self.nodes.is_empty() {
                write!(f, " ")?;
            }
        }
        write!(f, "}}")
    }
}

#[cfg(test)]
mod tests {
    use crate::{graph::Graph, node::Byte};

    #[test]
    fn insert_unique() {
        let g = &mut Graph::new();
        let id0 = Byte::Copy(0).insert(g);
        let id1 = Byte::Const(1).insert(g);
        let id2 = Byte::Add(id0, id1).insert(g);
        let id0b = Byte::Copy(0).insert(g);
        assert_ne!(id0, id0b);
        let id1b = Byte::Const(1).insert(g);
        let id2b = Byte::Add(id0, id1b).insert(g);
        assert_eq!(id1, id1b);
        assert_eq!(id2, id2b);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn compare_mixed_ids() {
        let g1 = &mut Graph::new();
        let g2 = &mut Graph::new();
        let id1 = Byte::Const(1).insert(g1);
        let id2 = Byte::Const(2).insert(g2);
        assert_eq!(id1.as_node_id().as_usize(), id2.as_node_id().as_usize());
        assert_ne!(id1.as_node_id(), id2.as_node_id());
        assert_ne!(id1, id2);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn insert_mixed_ids() {
        let g1 = &mut Graph::new();
        let g2 = &mut Graph::new();
        let id1 = Byte::Const(1).insert(g1);
        let id2 = Byte::Const(2).insert(g1);
        Byte::Add(id1, id2).insert(g2);
    }
}
