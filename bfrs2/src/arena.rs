use std::{
    collections::HashMap,
    fmt::{self, Debug, Formatter},
    hash::BuildHasher,
    num::NonZero,
    ops::{Deref, Index},
};

use crate::node::{BlockId, InputId, Node};

// TODO:
// - BUG: Hashes can have collisions. This needs to use HashTable.
// - Remove NodeId::from_index.

/// An arena of unique nodes, identified by ID.
pub struct Arena {
    /// Deduplicated nodes.
    nodes: Vec<Node>,
    /// Map from node hash to node ID for value numbering.
    ids: HashMap<u64, NodeId>,
    /// The ID of the next basic block.
    next_block: BlockId,
    /// The ID of the next unique input.
    next_input: InputId,
}

/// The value-numbered ID of a node in an arena.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(NonZero<u32>);

/// A recursive reference to a node in an arena.
#[derive(Clone, Copy)]
pub struct NodeRef<'a> {
    id: NodeId,
    arena: &'a Arena,
}

impl Arena {
    /// Constructs a new, empty arena.
    pub fn new() -> Self {
        Arena {
            nodes: Vec::new(),
            ids: HashMap::new(),
            next_block: BlockId(0),
            next_input: InputId(0),
        }
    }

    /// Inserts a node into this arena and returns its unique ID. The node must
    /// already be idealized. Any structurally equivalent nodes are deduplicated
    /// and receive the same ID.
    pub fn insert_ideal(&mut self, node: Node) -> NodeId {
        let hash = self.ids.hasher().hash_one(&node);
        if let Some(&id) = self.ids.get(&hash) {
            return id;
        }
        self.nodes.push(node);
        let len = u32::try_from(self.nodes.len()).expect("arena too large for u32 index");
        NodeId(NonZero::new(len).unwrap())
    }

    /// Generates a fresh ID for the next basic block.
    pub fn fresh_block_id(&mut self) -> BlockId {
        let id = self.next_block;
        self.next_block = BlockId(self.next_block.0 + 1);
        id
    }

    /// Inserts a `Node::Input` with a fresh ID.
    pub fn fresh_input(&mut self) -> NodeId {
        let id = self.insert_ideal(Node::Input(self.next_input));
        self.next_input = InputId(self.next_input.0 + 1);
        id
    }

    /// Gets a recursive reference to a node in the arena.
    pub fn get(&self, id: NodeId) -> NodeRef<'_> {
        NodeRef { id, arena: self }
    }
}

impl Index<NodeId> for Arena {
    type Output = Node;

    fn index(&self, id: NodeId) -> &Self::Output {
        &self.nodes[id.index()]
    }
}

impl NodeId {
    /// Constructs a node ID from an index.
    pub(crate) fn from_index(index: u32) -> Self {
        NodeId(NonZero::new(index + 1).unwrap())
    }

    /// Gets the index of the node in its arena.
    pub fn index(&self) -> usize {
        (self.0.get() - 1) as usize
    }
}

impl<'a> NodeRef<'a> {
    /// Gets the ID of this node.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Gets a reference to this node.
    pub fn node(&self) -> &'a Node {
        &self.arena[self.id]
    }

    /// Gets a reference to the arena which contains this node.
    pub fn arena(&self) -> &'a Arena {
        self.arena
    }

    /// Gets a recursive reference to a node in the arena.
    pub fn get(&self, id: NodeId) -> NodeRef<'a> {
        NodeRef {
            id,
            arena: self.arena,
        }
    }
}

impl Deref for NodeRef<'_> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node()
    }
}

impl Debug for Arena {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "Arena ")?;
        f.debug_map()
            .entries(self.nodes.iter().enumerate())
            .finish()
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NodeId").field(&self.index()).finish()
    }
}
