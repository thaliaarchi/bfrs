#[cfg(debug_assertions)]
use std::sync::atomic::{AtomicU32, Ordering};
use std::{
    fmt::{self, Debug, Formatter},
    hash::{BuildHasher, Hash, Hasher},
    num::NonZeroU32,
    ops::{Deref, Index, IndexMut},
    ptr,
};

use hashbrown::{hash_map::DefaultHashBuilder, HashTable};

use crate::node::Node;
#[cfg(debug_assertions)]
use crate::node::NodeType;

/// A graph of unique nodes, structured as an arena.
///
/// # Safety
///
/// It is undefined behavior to use a `NodeId` in any graph other than the one
/// which created it.
pub struct Graph {
    nodes: Vec<Node>,
    /// A table for deduplicating nodes. The key is the 0-based index of the
    /// node in `nodes`.
    table: HashTable<u32>,
    hash_builder: DefaultHashBuilder,
    #[cfg(debug_assertions)]
    graph_id: u32,
}

/// The ID of a node in a graph.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    /// The 1-based ID of the node, i.e., the index plus 1.
    node_id: NonZeroU32,
    /// The ID of the graph which contains the node, used for ensuring a
    /// `NodeId` is not used with a different graph when debug assertions are
    /// enabled.
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
        self.assert_node(&node);
        match &node {
            Node::Copy(_) | Node::Input { .. } => self.insert_unique(node),
            Node::Const(_) | Node::Add(..) | Node::Mul(..) | Node::Array(_) => {
                self.get_or_insert(node)
            }
        }
    }

    /// Inserts a node without deduplicating and returns its ID.
    fn insert_unique(&mut self, node: Node) -> NodeId {
        let id = NodeId::new(
            self.nodes.len(),
            #[cfg(debug_assertions)]
            self.graph_id,
        );
        self.nodes.push(node);
        id
    }

    /// Gets or inserts a node and returns its ID.
    fn get_or_insert(&mut self, node: Node) -> NodeId {
        let hash = self.hash_builder.hash_one(&node);
        let eq = |&index: &u32| {
            let key = unsafe { self.nodes.get_unchecked(index as usize) };
            &node == key
        };
        let hasher = |&index: &u32| {
            let key = unsafe { self.nodes.get_unchecked(index as usize) };
            self.hash_builder.hash_one(key)
        };
        let index = *self
            .table
            .entry(hash, eq, hasher)
            .or_insert_with(|| {
                let index = self.nodes.len();
                let _ = NodeId::new(
                    index,
                    #[cfg(debug_assertions)]
                    self.graph_id,
                );
                self.nodes.push(node);
                index as u32
            })
            .get();
        NodeId::new_unchecked(
            index,
            #[cfg(debug_assertions)]
            self.graph_id,
        )
    }

    /// Gets the ID of a node.
    pub fn find(&self, node: &Node) -> Option<NodeId> {
        self.assert_node(&node);
        let hash = self.hash_builder.hash_one(&node);
        let eq = |&index: &u32| {
            let key = unsafe { self.nodes.get_unchecked(index as usize) };
            node == key
        };
        let index = *self.table.find(hash, eq)?;
        Some(NodeId::new_unchecked(
            index,
            #[cfg(debug_assertions)]
            self.graph_id,
        ))
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

    fn assert_byte(&self, id: NodeId) {
        self.assert_id(id);
        assert!(
            matches!(self[id].ty(), NodeType::Byte),
            "node is not a byte",
        );
    }

    fn assert_node(&self, node: &Node) {
        match node {
            Node::Copy(_) | Node::Const(_) | Node::Input { .. } => {}
            &Node::Add(lhs, rhs) | &Node::Mul(lhs, rhs) => {
                self.assert_byte(lhs);
                self.assert_byte(rhs);
            }
            Node::Array(elements) => {
                for &id in elements {
                    self.assert_byte(id);
                }
            }
        }
    }
}

#[cfg(not(debug_assertions))]
impl Graph {
    fn assert_id(&self, _id: NodeId) {}
    fn assert_node(&self, _node: &Node) {}
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
    #[inline]
    fn new(index: usize, #[cfg(debug_assertions)] graph_id: u32) -> Self {
        if let Some(node_id) = u32::try_from(index)
            .ok()
            .and_then(|index| index.checked_add(1))
        {
            NodeId {
                node_id: unsafe { NonZeroU32::new_unchecked(node_id) },
                #[cfg(debug_assertions)]
                graph_id,
            }
        } else {
            #[inline(never)]
            #[cold]
            fn graph_overflow() -> ! {
                panic!("graph too large for u32 index");
            }
            graph_overflow();
        }
    }

    #[cfg(debug_assertions)]
    fn new_unchecked(index: u32, graph_id: u32) -> Self {
        Self::new(usize::try_from(index).unwrap(), graph_id)
    }

    #[cfg(not(debug_assertions))]
    fn new_unchecked(index: u32) -> Self {
        let node_id = unsafe { NonZeroU32::new_unchecked(index + 1) };
        NodeId { node_id }
    }

    /// Returns the index of this node ID.
    #[inline]
    pub fn as_usize(&self) -> usize {
        self.node_id.get() as usize - 1
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
            write!(f, "%{i} : {node:?}")
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
    use crate::{graph::Graph, node::Node};

    #[test]
    fn insert_unique() {
        let g = &mut Graph::new();
        let id0 = Node::Copy(0).insert(g);
        let id1 = Node::Const(1).insert(g);
        let id2 = Node::Add(id0, id1).insert(g);
        let id0b = Node::Copy(0).insert(g);
        assert_ne!(id0, id0b);
        let id1b = Node::Const(1).insert(g);
        let id2b = Node::Add(id0, id1b).insert(g);
        assert_eq!(id1, id1b);
        assert_eq!(id2, id2b);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn compare_mixed_ids() {
        let g1 = &mut Graph::new();
        let g2 = &mut Graph::new();
        let id1 = Node::Const(1).insert(g1);
        let id2 = Node::Const(2).insert(g2);
        assert_eq!(id1.as_usize(), id2.as_usize());
        assert_ne!(id1, id2);
    }

    #[cfg(debug_assertions)]
    #[test]
    #[should_panic]
    fn insert_mixed_ids() {
        let g1 = &mut Graph::new();
        let g2 = &mut Graph::new();
        let id1 = Node::Const(1).insert(g1);
        let id2 = Node::Const(2).insert(g1);
        Node::Add(id1, id2).insert(g2);
    }
}
