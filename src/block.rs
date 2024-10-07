use std::{collections::VecDeque, mem};

use crate::{
    arena::{Arena, NodeId},
    node::{BlockId, Node, Offset},
};

/// The memory and effects of a basic block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Block {
    /// The ID of this block, unique per arena.
    pub id: BlockId,
    /// The values modified in this memory.
    pub memory: VecDeque<Option<NodeId>>,
    /// The sequence of effects in this basic block.
    pub effects: Vec<Effect>,

    /// The relative offset of the cell pointer.
    pub offset: Offset,
    /// The relative offset of the first cell in `memory`.
    pub min_offset: Offset,
    /// The minimum offset left that has been guarded.
    pub guarded_left: Offset,
    /// The maximum offset right that has been guarded.
    pub guarded_right: Offset,
}

/// An observable effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Printing bytes.
    Output(Vec<NodeId>),
    /// Reading a byte from the user. The node is always `Node::Input`.
    Input(NodeId),
    /// Guarding that a shift can be performed by a certain amount.
    GuardShift(Offset),
}

/// A builder for a basic block, which avoids constructing intermediate nodes
/// from sequences of `+` and `-` instructions.
#[derive(Debug)]
pub struct BlockBuilder {
    /// The block under construction.
    block: Block,
    /// Constant addends to each cell. `addends[i]` corresponds with
    /// `block.memory[i]`.
    addends: VecDeque<u8>,
}

impl Block {
    /// Constructs a new, empty basic block.
    pub fn new(id: BlockId) -> Self {
        Block {
            id,
            memory: VecDeque::new(),
            effects: Vec::new(),
            offset: Offset(0),
            min_offset: Offset(0),
            guarded_left: Offset(0),
            guarded_right: Offset(0),
        }
    }

    /// Gets the value of the cell at the offset.
    pub fn get_cell(&self, offset: Offset) -> Option<NodeId> {
        self.memory
            .get(usize::try_from(offset.0 - self.min_offset.0).ok()?)
            .copied()
            .flatten()
    }

    /// The maximum offset, which has a modified value in this basic block.
    pub fn max_offset(&self) -> Offset {
        Offset(self.min_offset.0 + self.memory.len() as i64)
    }

    /// Returns an iterator for cells assigned in this block.
    pub fn iter_memory(&self) -> impl Iterator<Item = (Offset, NodeId)> + '_ {
        (self.min_offset.0..)
            .map(Offset)
            .zip(self.memory.iter())
            .filter_map(|(offset, cell)| cell.map(|cell| (offset, cell)))
    }

    /// Returns an iterator for mutable references to cells assigned in this
    /// block.
    pub fn iter_memory_mut(&mut self) -> impl Iterator<Item = (Offset, &mut NodeId)> + '_ {
        (self.min_offset.0..)
            .map(Offset)
            .zip(self.memory.iter_mut())
            .filter_map(|(offset, cell)| cell.as_mut().map(|cell| (offset, cell)))
    }
}

impl BlockBuilder {
    /// Constructs a new builder for basic blocks with successive IDs starting
    /// from 0.
    pub fn new() -> Self {
        BlockBuilder {
            block: Block::new(BlockId(0)),
            addends: VecDeque::new(),
        }
    }

    /// Shifts the cell pointer by a constant amount and guards that the shift
    /// is in bounds.
    pub fn shift(&mut self, delta: i64) {
        let block = &mut self.block;
        block.offset += delta;
        if block.offset < block.guarded_left {
            block.guarded_left = block.offset;
        } else if block.offset > block.guarded_right {
            block.guarded_right = block.offset;
        } else {
            return;
        };
        block.effects.push(Effect::GuardShift(block.offset));
    }

    /// Gets the value at the cell pointer, forcing construction of its nodes.
    pub fn get(&mut self, a: &mut Arena) -> NodeId {
        let (&mut base, &mut addend) = self.get_parts();
        let base = base.unwrap_or_else(|| Node::Copy(self.block.offset, self.block.id).insert(a));
        if addend != 0 {
            Node::Add(base, Node::Const(addend).insert(a)).insert(a)
        } else {
            base
        }
    }

    /// Sets the value at the cell pointer.
    pub fn set(&mut self, node: NodeId) {
        let (base, addend) = self.get_parts();
        *base = Some(node);
        *addend = 0;
    }

    /// Adds a constant amount to the value at the cell pointer.
    pub fn add(&mut self, addend: u8) {
        let (_base, addend1) = self.get_parts();
        *addend1 = addend1.wrapping_add(addend);
    }

    /// Outputs the value at the cell pointer.
    pub fn output(&mut self, a: &mut Arena) {
        let value = self.get(a);
        if let Some(Effect::Output(values)) = self.block.effects.last_mut() {
            values.push(value);
        } else {
            self.block.effects.push(Effect::Output(vec![value]));
        }
    }

    /// Gets a byte from the user and sets the value at the cell pointer to it.
    pub fn input(&mut self, a: &mut Arena) {
        let input = a.fresh_input();
        self.set(input);
        self.block.effects.push(Effect::Input(input));
    }

    /// Gets the base node (a `Copy` or `Input`) and constant addend for the
    /// value at the cell pointer.
    fn get_parts(&mut self) -> (&mut Option<NodeId>, &mut u8) {
        let block = &mut self.block;
        let i = block.offset.0 - block.min_offset.0;
        if block.memory.is_empty() {
            block.memory.push_back(None);
            self.addends.push_back(0);
            block.min_offset = block.offset;
        } else if i < 0 {
            let additional = -i as usize;
            block.memory.reserve(additional);
            for _ in 0..additional {
                block.memory.push_front(None);
            }
            self.addends.reserve(additional);
            for _ in 0..additional {
                self.addends.push_front(0);
            }
            block.min_offset = block.offset;
        } else if i as usize >= block.memory.len() {
            block.memory.resize(i as usize + 1, None);
            self.addends.resize(i as usize + 1, 0);
        }
        let i = (block.offset.0 - block.min_offset.0) as usize;
        (&mut block.memory[i], &mut self.addends[i])
    }

    /// Returns the finished basic block.
    pub fn finish(&mut self, a: &mut Arena) -> Block {
        let next_block = Block::new(BlockId(self.block.id.0 + 1));
        let mut block = mem::replace(&mut self.block, next_block);
        for i in 0..block.memory.len() {
            let node = &mut block.memory[i];
            let addend = self.addends[i];
            if addend != 0 {
                let base = node
                    .unwrap_or_else(|| Node::Copy(block.min_offset + i as i64, block.id).insert(a));
                *node = Some(Node::Add(base, Node::Const(addend).insert(a)).insert(a));
            }
        }
        self.addends.clear();
        block
    }

    /// Reports whether the basic block being constructed is empty.
    pub fn is_empty(&self) -> bool {
        let block = &self.block;
        block.memory.is_empty()
            && block.effects.is_empty()
            && block.offset == Offset(0)
            && block.min_offset == Offset(0)
            && block.guarded_left == Offset(0)
            && block.guarded_right == Offset(0)
    }
}
