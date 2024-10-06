use std::{
    collections::VecDeque,
    fmt::{self, Debug, Formatter},
};

use crate::{
    graph::{Graph, NodeId},
    node::Node,
    region::Effect,
};

/// Abstract model of memory. It is constructed with `MemoryBuilder`.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct Memory {
    /// The values modified in this memory.
    memory: VecDeque<Option<NodeId>>,
    /// The relative offset of the cell pointer.
    offset: isize,
    /// The relative offset of the first cell in `memory`.
    min_offset: isize,
    /// The minimum shift left that has been guarded.
    guarded_left: isize,
    /// The maximum shift right that has been guarded.
    guarded_right: isize,
}

/// A builder for any number of `Memory`s, which constructs nodes on demand.
#[derive(Clone, Debug)]
pub struct MemoryBuilder {
    /// Tuples of the base node (a `Copy` or `Input`) and constant addend for
    /// each cell in this memory.
    memory: VecDeque<(Option<NodeId>, u8)>,
    offset: isize,
    min_offset: isize,
    guarded_left: isize,
    guarded_right: isize,
}

impl Memory {
    /// Gets the cell at the offset.
    pub fn get_cell(&self, offset: isize) -> Option<NodeId> {
        self.memory
            .get(usize::try_from(offset - self.min_offset).ok()?)
            .copied()
            .flatten()
    }

    /// Gets a mutable reference to the cell at the offset.
    pub fn get_cell_mut(&mut self, offset: isize) -> &mut Option<NodeId> {
        self.reserve(offset, offset + 1);
        &mut self.memory[(offset - self.min_offset) as usize]
    }

    /// Gets the value at the offset, forcing a `Copy` if this cell had not been
    /// modified.
    pub fn compute_cell(&mut self, offset: isize, g: &Graph) -> NodeId {
        self.reserve(offset, offset + 1);
        let i = (offset - self.min_offset) as usize;
        match &mut self.memory[i] {
            Some(cell) => *cell,
            cell @ None => {
                let copy = Node::Copy(offset).insert(g);
                *cell = Some(copy);
                copy
            }
        }
    }

    /// The offset of the cell pointer relative to this memory.
    pub fn offset(&self) -> isize {
        self.offset
    }

    /// The minimum relative offset, which has cells assigned in this memory.
    pub fn min_offset(&self) -> isize {
        self.min_offset
    }

    /// The maximum relative offset, which has cells assigned in this memory.
    pub fn max_offset(&self) -> isize {
        self.min_offset + self.memory.len() as isize
    }

    /// The minimum shift left that has been guarded.
    pub fn guarded_left(&self) -> isize {
        self.guarded_left
    }

    /// The maximum shift right that has been guarded.
    pub fn guarded_right(&self) -> isize {
        self.guarded_right
    }

    /// Shifts the cell pointer by a given amount. Panics if this offset has not
    /// been guarded.
    pub fn shift(&mut self, amount: isize) {
        self.offset += amount;
        assert!(self.guarded_left <= self.offset && self.offset < self.guarded_right);
    }

    /// Guards for a shift to a given offset. Returns whether a new guard effect
    /// is needed.
    pub fn guard_offset(&mut self, offset: isize) -> bool {
        if offset < self.guarded_left {
            self.guarded_left = offset;
            true
        } else if offset > self.guarded_right {
            self.guarded_right = offset;
            true
        } else {
            false
        }
    }

    /// Composes two memories by applying the operations of `other` to `self`.
    /// The values in `other` must have already been rebased.
    pub(super) fn apply(&mut self, other: &Self) {
        let min_offset = self.offset + other.min_offset;
        let max_offset = self.offset + other.max_offset();
        self.reserve(min_offset, max_offset);
        for (i, &cell) in other.memory.iter().enumerate() {
            if let Some(cell) = cell {
                self.memory[(min_offset - self.min_offset) as usize + i] = Some(cell);
            }
        }
        self.guarded_left = self.guarded_left.min(self.offset + other.guarded_left);
        self.guarded_right = self.guarded_right.max(self.offset + other.guarded_right);
        self.offset += other.offset;
    }

    /// Reserves slots for cells in the range `min_offset..max_offset` and fills
    /// them with `None`.
    fn reserve(&mut self, min_offset: isize, max_offset: isize) {
        debug_assert!(max_offset >= min_offset);
        if self.memory.is_empty() {
            self.memory.resize((max_offset - min_offset) as usize, None);
            self.min_offset = min_offset;
            return;
        }
        let min_offset = self.min_offset.min(min_offset);
        let max_offset = self.max_offset().max(max_offset);
        let len = (max_offset - min_offset) as usize;
        self.memory.reserve(len - self.memory.len());
        for _ in 0..(self.min_offset - min_offset) as usize {
            self.memory.push_front(None);
        }
        self.memory.resize(len, None);
        self.min_offset = min_offset;
    }

    /// Returns an iterator for values in this memory.
    pub fn iter(&self) -> impl Iterator<Item = (isize, NodeId)> + '_ {
        (self.min_offset..)
            .zip(self.memory.iter())
            .filter_map(|(offset, cell)| cell.map(|cell| (offset, cell)))
    }

    /// Returns an iterator for mutable references to values in this memory.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (isize, &mut NodeId)> + '_ {
        (self.min_offset..)
            .zip(self.memory.iter_mut())
            .filter_map(|(offset, cell)| cell.as_mut().map(|cell| (offset, cell)))
    }
}

impl Debug for Memory {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        struct MemoryData<'a>(&'a Memory);
        impl Debug for MemoryData<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.debug_map().entries(self.0.iter()).finish()
            }
        }
        f.debug_struct("Memory")
            .field("memory", &MemoryData(&self))
            .field("offset", &self.offset)
            .field("guarded", &(self.guarded_left..self.guarded_right))
            .finish()
    }
}

impl MemoryBuilder {
    /// Constructs an empty builder for any number of `Memory`s.
    pub fn new() -> Self {
        MemoryBuilder {
            memory: VecDeque::new(),
            min_offset: 0,
            offset: 0,
            guarded_left: 0,
            guarded_right: 0,
        }
    }

    /// Gets the value at the cell pointer, forcing construction of its nodes.
    pub fn compute_cell(&mut self, g: &Graph) -> NodeId {
        let (base, addend) = *self.get_cell_parts();
        let base = base.unwrap_or_else(|| Node::Copy(self.offset).insert(g));
        if addend != 0 {
            Node::Add(base, Node::Const(addend).insert(g)).insert(g)
        } else {
            base
        }
    }

    /// Sets the value at the cell pointer.
    pub fn set_cell(&mut self, cell: NodeId) {
        *self.get_cell_parts() = (Some(cell), 0);
    }

    /// Adds a constant amount to the value at the cell pointer.
    pub fn add(&mut self, addend: u8) {
        let (_, addend1) = self.get_cell_parts();
        *addend1 = addend1.wrapping_add(addend);
    }

    /// Gets the base node (a `Copy` or `Input`) and constant addend for the
    /// value at the cell pointer.
    fn get_cell_parts(&mut self) -> &mut (Option<NodeId>, u8) {
        if self.memory.is_empty() {
            self.memory.push_back((None, 0));
            self.min_offset = self.offset;
        } else if self.offset < self.min_offset {
            let n = (self.min_offset - self.offset) as usize;
            self.memory.reserve(n);
            for _ in 0..n {
                self.memory.push_front((None, 0));
            }
            self.min_offset = self.offset;
        } else if self.offset >= self.min_offset + self.memory.len() as isize {
            self.memory
                .resize((self.offset - self.min_offset + 1) as usize, (None, 0));
        }
        &mut self.memory[(self.offset - self.min_offset) as usize]
    }

    /// Shifts the cell pointer by a given amount. Returns a guard effect if
    /// this offset has not been guarded.
    pub fn shift(&mut self, amount: isize) -> Option<Effect> {
        self.offset += amount;
        if self.offset < self.guarded_left {
            self.guarded_left = self.offset;
            Some(Effect::GuardShift(self.offset))
        } else if self.offset > self.guarded_right {
            self.guarded_right = self.offset;
            Some(Effect::GuardShift(self.offset))
        } else {
            None
        }
    }

    /// Resets the builder to start constructing a new `Memory`.
    pub fn reset(&mut self) {
        self.memory.clear();
        self.min_offset = 0;
        self.offset = 0;
        self.guarded_left = 0;
        self.guarded_right = 0;
    }

    /// Builds the `Memory`, forcing construction of the used nodes.
    pub fn finish(&mut self, g: &Graph) -> Memory {
        let mut memory = VecDeque::with_capacity(self.memory.len());
        for (&(base, addend), offset) in self.memory.iter().zip(self.min_offset..) {
            let base = match base {
                Some(base) => Some(base),
                None if addend != 0 => Some(Node::Copy(offset).insert(g)),
                None => None,
            };
            memory.push_back(base.map(|base| {
                if addend != 0 {
                    Node::Add(base, Node::Const(addend).insert(g)).insert(g)
                } else {
                    base
                }
            }));
        }
        Memory {
            memory,
            offset: self.offset,
            min_offset: self.min_offset,
            guarded_left: self.guarded_left,
            guarded_right: self.guarded_right,
        }
    }
}
