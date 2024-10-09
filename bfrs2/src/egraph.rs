//! E-graph types.

use std::{
    fmt::{self, Debug, Formatter},
    hash::BuildHasher,
    hint::unreachable_unchecked,
    mem,
    num::NonZero,
    ops::{Deref, Index},
};

use hashbrown::{DefaultHashBuilder, HashTable};

use crate::node::{BlockId, InputId, Node};

// TODO:
// - Compare performance of updating nodes in `Eclass::nodes` to point to the
//   replaced e-class with the current tree of e-classes. The user could still
//   have an `EclassId` to a replaced e-class, so public APIs would still need
//   the forwarding loop.
// - Build debugging infrastructure to visualize nodes in e-classes.

/// An e-graph. Structurally identical nodes receive the same ID. Semantically
/// equivalent nodes are in the same e-class and considered interchangeable.
/// Rather than mutating nodes, fresh nodes are inserted and unioned with the
/// old using [`NodeId::replace`].
///
/// Note that unlike egg and egglog, this does not use equality saturation and
/// rewrites are written ad hoc.
pub struct Graph {
    /// Deduplicated nodes, identified by `NodeId`.
    nodes: Vec<NodeEntry>, // NodeId -> NodeEntry
    /// Map from node to node index for value numbering.
    node_indices: HashTable<u32>, // Node -> NodeId index
    /// E-classes, identified by `EclassId`.
    eclasses: Vec<EclassEntry>, // EclassId -> EclassEntry
    /// Hasher for `node_ids`.
    hash_builder: DefaultHashBuilder,
    /// The ID of the next basic block.
    next_block: BlockId,
    /// The ID of the next unique input.
    next_input: InputId,
    /// The optimization pass which is currently executing.
    pass: Pass,
}

/// A node entry in the `Graph`, which knows its e-class and the pass which
/// created it. Nodes are not modified after insertion.
pub struct NodeEntry {
    node: Node,
    hash: u64,
    eclass: Option<EclassId>,
    creator: Pass,
}

/// The value-numbered ID of a node in an e-graph.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(NonZero<u32>);

/// A recursive reference to a node in an e-graph.
#[derive(Clone, Copy)]
pub struct NodeRef<'g> {
    id: NodeId,
    graph: &'g Graph,
}

/// An equivalence class of nodes, i.e., a set of nodes, which are equivalent
/// and can be used interchangeably. One node is selected as the canonical
/// representation of the e-class.
#[derive(Debug)]
pub struct Eclass {
    canon: NodeId,
    nodes: Vec<NodeId>,
}

/// An e-class entry.
enum EclassEntry {
    /// The data for the e-class.
    Eclass(Eclass),
    /// This e-class has been unioned with another and now forwards to that
    /// e-class. A chain of `Union` always ends with `Eclass`.
    Union(EclassId),
}

/// The ID of an e-class in an e-graph. An e-class can have multiple IDs.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct EclassId(NonZero<u32>);

/// An optimization pass.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pass {
    /// Parsing
    Parse,
    /// Add-loop to closed form multiply transformation
    AddLoopToMul,
    /// Loop quasi-invariant code motion through peeling
    QuasiInvariantPeel,
    /// Constant copy propagation.
    CopyConst,
    /// An unknown pass.
    Unknown,
}

impl Graph {
    /// Constructs a new, empty e-graph.
    #[inline]
    pub fn new() -> Self {
        Graph {
            nodes: Vec::new(),
            node_indices: HashTable::new(),
            eclasses: Vec::new(),
            hash_builder: DefaultHashBuilder::default(),
            next_block: BlockId(0),
            next_input: InputId(0),
            pass: Pass::Unknown,
        }
    }

    /// Inserts this node into the e-graph and places it in a singleton e-class.
    /// Structurally equivalent nodes are deduplicated and receive the same
    /// `NodeId`.
    pub fn insert(&mut self, node: Node) -> NodeId {
        self.assert_node(&node);
        let hash = self.hash_builder.hash_one(&node);
        let entry = self.node_indices.entry(
            hash,
            |&index| {
                // SAFETY: The length of `self.nodes` monotonically increases,
                // so if an index was in bounds on insertion, it remains in
                // bounds.
                let entry = unsafe { self.nodes.get_unchecked(index as usize) };
                entry.node == node
            },
            |&index| {
                // SAFETY: Same as above.
                let entry = unsafe { self.nodes.get_unchecked(index as usize) };
                entry.hash
            },
        );
        let index = entry.or_insert_with(|| {
            self.nodes.push(NodeEntry {
                node,
                hash,
                eclass: None,
                creator: self.pass,
            });
            let Ok(id) = u32::try_from(self.nodes.len()) else {
                Self::node_overflow()
            };
            id - 1
        });
        // SAFETY: Indexes are only constructed above and are guaranteed to be
        // non-zero.
        unsafe { NodeId::from_index(*index.get()) }
    }

    /// Looks up the ID of this node, if it has already been inserted.
    pub fn find(&self, node: &Node) -> Option<NodeId> {
        let hash = self.hash_builder.hash_one(node);
        let index = self.node_indices.find(hash, |&index| {
            // SAFETY: Same as `Graph::insert`.
            let entry = unsafe { self.nodes.get_unchecked(index as usize) };
            &entry.node == node
        });
        // SAFETY: Same as `Graph::insert`.
        index.map(|&index| unsafe { NodeId::from_index(index) })
    }

    /// Generates a fresh ID for the next basic block.
    pub fn fresh_block_id(&mut self) -> BlockId {
        let id = self.next_block;
        self.next_block = BlockId(self.next_block.0 + 1);
        id
    }

    /// Inserts a `Node::Input` with a fresh ID.
    pub fn fresh_input(&mut self) -> NodeId {
        let id = self.insert(Node::Input(self.next_input));
        self.next_input = InputId(self.next_input.0 + 1);
        id
    }

    /// Gets a recursive reference to a node in the e-graph.
    pub fn get(&self, id: NodeId) -> NodeRef<'_> {
        NodeRef { id, graph: self }
    }

    /// Gets the entry for a node.
    #[inline]
    pub fn entry(&self, id: NodeId) -> &NodeEntry {
        self.assert_node_id(id);
        &self.nodes[id.index()]
    }

    /// Gets a reference to the root e-class for this ID.
    fn eclass(&self, mut eid: EclassId) -> (EclassId, &Eclass) {
        self.assert_eclass_id(eid);
        loop {
            match unsafe { self.eclasses.get_unchecked(eid.index()) } {
                EclassEntry::Eclass(eclass) => return (eid, eclass),
                EclassEntry::Union(eid2) => eid = *eid2,
            }
        }
    }

    /// Gets a mutable reference to the root e-class for this ID.
    ///
    /// # Safety
    ///
    /// It may only be used with an `EclassId` from itself.
    fn eclass_mut(eclasses: &mut Vec<EclassEntry>, mut eid: EclassId) -> (EclassId, &mut Eclass) {
        // TODO: Simplify once Polonius is stable.
        debug_assert!(eid.index() < eclasses.len());
        unsafe {
            while let EclassEntry::Union(eid2) = eclasses.get_unchecked(eid.index()) {
                eid = *eid2;
            }
            let EclassEntry::Eclass(eclass) = eclasses.get_unchecked_mut(eid.index()) else {
                unreachable_unchecked()
            };
            (eid, eclass)
        }
    }

    /// Unifies `origin` and `canon` into the same e-class and makes `canon` the
    /// canonical node.
    fn replace(&mut self, origin: NodeId, canon: NodeId) {
        self.assert_node_id(origin);
        self.assert_node_id(canon);
        let eid1 = self.nodes[origin.index()].eclass;
        let eid2 = self.nodes[canon.index()].eclass;
        match (eid1, eid2) {
            (Some(eid1), Some(eid2)) => {
                let (eid1, eclass1) = Graph::eclass_mut(&mut self.eclasses, eid1);
                let eclass1 = eclass1 as *mut Eclass;
                let (eid2, eclass2) = Graph::eclass_mut(&mut self.eclasses, eid2);
                let eclass2 = eclass2 as *mut Eclass;
                self.nodes[origin.index()].eclass = Some(eid2);
                // Update to the eclass root, so future accesses seek less.
                self.nodes[canon.index()].eclass = Some(eid2);
                if eid1 == eid2 {
                    return;
                }
                // SAFETY: The indices do not alias, so the values do not alias.
                let (eclass1, eclass2) = unsafe { (&mut *eclass1, &mut *eclass2) };
                eclass2.canon = canon;
                if eclass2.nodes.len() >= eclass1.nodes.len() {
                    eclass2.nodes.extend_from_slice(&eclass1.nodes);
                } else {
                    let mut nodes = mem::take(&mut eclass1.nodes);
                    nodes.extend_from_slice(&eclass2.nodes);
                    eclass2.nodes = nodes;
                }
                self.eclasses[eid1.index()] = EclassEntry::Union(eid2);
            }
            (Some(eid), None) | (None, Some(eid)) => {
                let (eid, eclass) = Graph::eclass_mut(&mut self.eclasses, eid);
                self.nodes[origin.index()].eclass = Some(eid);
                self.nodes[canon.index()].eclass = Some(eid);
                eclass.canon = canon;
                eclass
                    .nodes
                    .push(if eid1.is_none() { origin } else { canon });
            }
            (None, None) => {
                self.eclasses.push(EclassEntry::Eclass(Eclass {
                    canon,
                    nodes: vec![canon, origin],
                }));
                let Ok(eid) = u32::try_from(self.eclasses.len()) else {
                    Graph::eclass_overflow()
                };
                let eid = EclassId(NonZero::new(eid).unwrap());
                self.nodes[origin.index()].eclass = Some(eid);
                self.nodes[canon.index()].eclass = Some(eid);
            }
        }
    }

    /// Returns the number of nodes in this e-graph.
    #[inline]
    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    /// Returns whether this e-graph contains no values.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// Returns the number of e-classes in this e-graph.
    #[inline]
    pub fn eclasses(&self) -> usize {
        self.eclasses.len()
    }

    /// Records the optimization pass which is currently executing, so it can be
    /// reported in nodes that are generated by it.
    #[inline]
    pub fn set_pass(&mut self, pass: Pass) {
        self.pass = pass;
    }

    fn assert_node(&self, node: &Node) {
        match *node {
            Node::Copy(..) | Node::Const(_) | Node::Input(_) => {}
            Node::Add(lhs, rhs) | Node::Mul(lhs, rhs) => {
                self.assert_node_id(lhs);
                self.assert_node_id(rhs);
            }
        }
    }

    #[inline]
    fn assert_node_id(&self, id: NodeId) {
        if id.index() >= self.nodes.len() {
            Self::bad_node_id();
        }
    }

    #[inline]
    fn assert_eclass_id(&self, eid: EclassId) {
        if eid.index() >= self.eclasses.len() {
            Self::bad_eclass_id();
        }
    }

    #[cold]
    #[inline(never)]
    fn node_overflow() -> ! {
        panic!("graph has too many nodes for u32 index");
    }

    #[cold]
    #[inline(never)]
    fn eclass_overflow() -> ! {
        panic!("graph has too many e-classes for u32 index");
    }

    #[cold]
    #[inline(never)]
    fn bad_node_id() -> ! {
        panic!("NodeId used in another Graph");
    }

    #[cold]
    #[inline(never)]
    fn bad_eclass_id() -> ! {
        panic!("EclassId used in another Graph");
    }
}

impl Index<NodeId> for Graph {
    type Output = Node;

    #[inline]
    fn index(&self, id: NodeId) -> &Self::Output {
        &self.nodes[id.index()].node
    }
}

impl Index<EclassId> for Graph {
    type Output = Eclass;

    #[inline]
    fn index(&self, eid: EclassId) -> &Self::Output {
        self.eclass(eid).1
    }
}

impl Default for Graph {
    fn default() -> Self {
        Graph::new()
    }
}

impl NodeEntry {
    /// Gets a reference to this node.
    #[inline]
    pub fn node(&self) -> &Node {
        &self.node
    }

    /// The e-class this node is in, or `None` if it is in a singleton e-class.
    #[inline]
    pub fn eclass(&self) -> Option<EclassId> {
        self.eclass
    }

    /// The pass which created this node.
    #[inline]
    pub fn creator(&self) -> Pass {
        self.creator
    }
}

impl NodeId {
    /// Constructs a node ID from an index.
    ///
    /// # Safety
    ///
    /// The caller must guarantee that `index` is non-zero.
    #[inline]
    unsafe fn from_index(index: u32) -> Self {
        NodeId(unsafe { NonZero::new_unchecked(index + 1) })
    }

    /// Returns the 0-based index of this ID.
    pub fn index(&self) -> usize {
        (self.0.get() - 1) as usize
    }

    /// Unifies `self` and `canon` into the same e-class and makes `canon` the
    /// canonical node.
    pub fn replace(self, canon: NodeId, g: &mut Graph) {
        g.replace(self, canon);
    }
}

impl<'g> NodeRef<'g> {
    /// Gets the ID of this node.
    pub fn id(&self) -> NodeId {
        self.id
    }

    /// Gets a reference to this node.
    pub fn node(&self) -> &'g Node {
        &self.graph[self.id]
    }

    /// Gets a reference to the e-graph which contains this node.
    pub fn graph(&self) -> &'g Graph {
        self.graph
    }

    /// Gets a recursive reference to a node in the e-graph.
    pub fn get(&self, id: NodeId) -> NodeRef<'g> {
        NodeRef {
            id,
            graph: self.graph,
        }
    }
}

impl Deref for NodeRef<'_> {
    type Target = Node;

    fn deref(&self) -> &Self::Target {
        self.node()
    }
}

impl Eclass {
    /// The canonical node which represents this e-class.
    #[inline]
    pub fn canon(&self) -> NodeId {
        self.canon
    }

    /// All nodes which are in this e-class.
    #[inline]
    pub fn nodes(&self) -> &[NodeId] {
        &self.nodes
    }
}

impl EclassId {
    /// Returns the 0-based index of this ID.
    #[inline]
    fn index(self) -> usize {
        self.0.get() as usize - 1
    }
}

impl Debug for Graph {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        struct SliceMap<'a, T>(&'a [T]);
        impl<T: Debug> Debug for SliceMap<'_, T> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.debug_map().entries(self.0.iter().enumerate()).finish()
            }
        }
        f.debug_struct("Graph")
            .field("nodes", &SliceMap(&self.nodes))
            .field("eclasses", &SliceMap(&self.eclasses))
            .finish()
    }
}

impl Debug for NodeEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("NodeEntry")
            .field("node", &self.node)
            .field("eclass", &self.eclass)
            .field("creator", &self.creator)
            .finish()
    }
}

impl Debug for NodeId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("NodeId").field(&self.index()).finish()
    }
}

impl Debug for EclassEntry {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            EclassEntry::Eclass(eclass) => f.debug_tuple("Eclass").field(&eclass).finish(),
            EclassEntry::Union(eid) => f.debug_tuple("Union").field(&eid).finish(),
        }
    }
}

impl Debug for EclassId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_tuple("EclassId").field(&self.index()).finish()
    }
}
