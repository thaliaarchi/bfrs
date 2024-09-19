#[cfg(not(debug_assertions))]
use std::hint::unreachable_unchecked;
use std::{
    fmt::{self, Debug, Formatter},
    ops::{Index, IndexMut},
};

use crate::{
    graph::{Graph, NodeId, NodeRef},
    node::{Array, Byte, Node},
};

macro_rules! define_id((
    $NodeTy:ident, $IdTy:ident, $as_node:ident,
    $indefinite_article:literal, $name:literal
) => {
    /// The ID of a
    #[doc = $name]
    /// node in a graph.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct $IdTy(pub(super) NodeId);

    impl $IdTy {
        /// Gets the untyped ID for this node.
        #[inline]
        pub fn as_node_id(self) -> NodeId {
            self.0
        }

        /// Gets a reference to this
        #[doc = $name]
        /// node.
        pub fn get<'g>(self, g: &'g Graph) -> NodeRef<'g, $IdTy> {
            g.get(self)
        }
    }

    impl Debug for $IdTy {
        fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
            Debug::fmt(&self.0, f)
        }
    }

    impl NodeId {
        /// Downcasts to
        #[doc = concat!($indefinite_article, " ", $name)]
        /// ID if the node is
        #[doc = concat!($indefinite_article, " ", $name, ".")]
        pub fn $as_node(self, g: &Graph) -> Option<ByteId> {
            if let Node::Byte(_) = g[self] {
                Some(ByteId(self))
            } else {
                None
            }
        }
    }

    impl Index<$IdTy> for Graph {
        type Output = $NodeTy;

        fn index(&self, id: $IdTy) -> &Self::Output {
            if let Node::$NodeTy(node) = &self[id.0] {
                node
            } else {
                unreachable_type()
            }
        }
    }

    impl IndexMut<$IdTy> for Graph {
        fn index_mut(&mut self, id: $IdTy) -> &mut Self::Output {
            if let Node::$NodeTy(node) = &mut self[id.0] {
                node
            } else {
                unreachable_type()
            }
        }
    }
});

define_id!(Byte, ByteId, as_byte_id, "a", "byte");
define_id!(Array, ArrayId, as_array_id, "an", "array");

/// The ID of a node in a graph, tagged with its type.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum TypedNodeId {
    Byte(ByteId),
    Array(ArrayId),
}

impl NodeId {
    /// Tags this ID with its type.
    pub fn with_type(self, g: &Graph) -> TypedNodeId {
        match g[self] {
            Node::Byte(_) => TypedNodeId::Byte(ByteId(self)),
            Node::Array(_) => TypedNodeId::Array(ArrayId(self)),
        }
    }
}

#[inline(always)]
#[cfg(not(debug_assertions))]
fn unreachable_type() -> ! {
    // SAFETY: Nodes are exclusively constructed and accessed through typed
    // indices, so cannot be dynamically mistyped.
    unsafe {
        unreachable_unchecked();
    }
}

#[inline(never)]
#[cfg(debug_assertions)]
fn unreachable_type() -> ! {
    unreachable!("node accessed with incorrect index type");
}
