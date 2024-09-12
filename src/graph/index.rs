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

/// The ID of a byte node in a graph.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ByteId(pub(super) NodeId);

/// The ID of an array node in a graph.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct ArrayId(pub(super) NodeId);

impl ByteId {
    /// Gets the untyped ID for this node.
    #[inline]
    pub fn as_node_id(&self) -> NodeId {
        self.0
    }

    /// Gets a reference to this byte node.
    pub fn get<'g>(&self, g: &'g Graph) -> NodeRef<'g, ByteId> {
        g.get(*self)
    }
}

impl ArrayId {
    /// Gets the untyped ID for this node.
    #[inline]
    pub fn as_node_id(&self) -> NodeId {
        self.0
    }

    /// Gets a reference to this arraynode.
    pub fn get<'g>(&self, g: &'g Graph) -> NodeRef<'g, ArrayId> {
        g.get(*self)
    }
}

impl Index<ByteId> for Graph {
    type Output = Byte;

    fn index(&self, id: ByteId) -> &Self::Output {
        if let Node::Byte(byte) = &self[id.0] {
            byte
        } else {
            unreachable_type()
        }
    }
}

impl IndexMut<ByteId> for Graph {
    fn index_mut(&mut self, id: ByteId) -> &mut Self::Output {
        if let Node::Byte(byte) = &mut self[id.0] {
            byte
        } else {
            unreachable_type()
        }
    }
}

impl Index<ArrayId> for Graph {
    type Output = Array;

    fn index(&self, id: ArrayId) -> &Self::Output {
        if let Node::Array(array) = &self[id.0] {
            array
        } else {
            unreachable_type()
        }
    }
}

impl IndexMut<ArrayId> for Graph {
    fn index_mut(&mut self, id: ArrayId) -> &mut Self::Output {
        if let Node::Array(array) = &mut self[id.0] {
            array
        } else {
            unreachable_type()
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

impl Debug for ByteId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}

impl Debug for ArrayId {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Debug::fmt(&self.0, f)
    }
}
