use std::{
    collections::{HashMap, VecDeque},
    mem,
};

use crate::{
    arena::{Arena, NodeId},
    node::{BlockId, InputId, Node, Offset},
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
            .get(offset.try_index_from(self.min_offset)?)
            .copied()
            .flatten()
    }

    /// The maximum offset, which has a modified value in this basic block.
    pub fn max_offset(&self) -> Offset {
        self.min_offset + self.memory.len() as i64
    }

    /// Concatenates two basic blocks. Applies the operations of `other` to
    /// `self`.
    pub fn concat(&mut self, other: &Self, a: &mut Arena) {
        self.effects.reserve(other.effects.len());
        for effect in &other.effects {
            let effect = match effect {
                Effect::Output(values) => {
                    let mut effects = mem::take(&mut self.effects);
                    let values = values
                        .iter()
                        .map(|value| value.move_to_block(other.id, self, a));
                    if let Some(Effect::Output(values1)) = effects.last_mut() {
                        values1.extend(values);
                    } else {
                        effects.push(Effect::Output(values.collect()));
                    }
                    self.effects = effects;
                    continue;
                }
                &Effect::Input(input) => Effect::Input(input),
                &Effect::GuardShift(offset) => {
                    let offset = offset + self.offset;
                    if offset < self.guarded_left {
                        self.guarded_left = offset;
                    } else if offset > self.guarded_right {
                        self.guarded_right = offset;
                    } else {
                        continue;
                    }
                    Effect::GuardShift(offset)
                }
            };
            self.effects.push(effect);
        }

        let mut other_memory = other.memory.clone();
        for cell in &mut other_memory {
            if let Some(cell) = cell {
                *cell = cell.move_to_block(other.id, self, a);
            }
        }
        let min_offset = self.offset + other.min_offset;
        let max_offset = self.offset + other.max_offset();
        self.reserve(min_offset, max_offset);
        for (i, &cell) in other_memory.iter().enumerate() {
            if let Some(cell) = cell {
                self.memory[min_offset.index_from(self.min_offset) + i] = Some(cell);
            }
        }
        self.guarded_left = self.guarded_left.min(self.offset + other.guarded_left);
        self.guarded_right = self.guarded_right.max(self.offset + other.guarded_right);
        self.offset += other.offset;
    }

    /// Reserves slots for cells in the range `min_offset..max_offset` and fills
    /// them with `None`.
    fn reserve(&mut self, min_offset: Offset, max_offset: Offset) {
        debug_assert!(max_offset >= min_offset);
        if self.memory.is_empty() {
            self.memory.resize(max_offset.index_from(min_offset), None);
            self.min_offset = min_offset;
            return;
        }
        let min_offset = self.min_offset.min(min_offset);
        let max_offset = self.max_offset().max(max_offset);
        let len = max_offset.index_from(min_offset);
        self.memory.reserve(len - self.memory.len());
        for _ in 0..self.min_offset.index_from(min_offset) {
            self.memory.push_front(None);
        }
        self.memory.resize(len, None);
        self.min_offset = min_offset;
    }

    /// Clones this block, making its copies be relative to the given block and
    /// generating fresh inputs.
    pub fn clone_fresh(&self, a: &mut Arena) -> Self {
        let id = a.fresh_block_id();
        let mut inputs = HashMap::new();
        let memory = self
            .memory
            .iter()
            .map(|cell| cell.map(|cell| cell.clone_in_block(self.id, id, &mut inputs, a)))
            .collect();
        let effects = self
            .effects
            .iter()
            .map(|effect| effect.clone_in_block(self.id, id, &mut inputs, a))
            .collect();
        Block {
            id,
            memory,
            effects,
            offset: self.offset,
            min_offset: self.min_offset,
            guarded_left: self.guarded_left,
            guarded_right: self.guarded_right,
        }
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
        let i = block.offset.index_from_signed(block.min_offset);
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
        let next_block = Block::new(BlockId(0));
        let mut block = mem::replace(&mut self.block, next_block);
        block.id = a.fresh_block_id();
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

impl NodeId {
    /// Makes a copy of this node, but with its copies be relative to the given
    /// block.
    pub fn move_to_block(self, block_from: BlockId, block_to: &Block, a: &mut Arena) -> Self {
        match a[self] {
            Node::Copy(offset, block) if block == block_from => {
                let offset = block_to.offset + offset;
                block_to
                    .get_cell(offset)
                    .unwrap_or_else(|| Node::Copy(offset, block_to.id).insert_ideal(a))
            }
            Node::Copy(..) | Node::Const(_) | Node::Input(_) => self,
            Node::Add(lhs, rhs) => {
                let lhs = lhs.move_to_block(block_from, block_to, a);
                let rhs = rhs.move_to_block(block_from, block_to, a);
                Node::Add(lhs, rhs).insert(a)
            }
            Node::Mul(lhs, rhs) => {
                let lhs = lhs.move_to_block(block_from, block_to, a);
                let rhs = rhs.move_to_block(block_from, block_to, a);
                Node::Mul(lhs, rhs).insert(a)
            }
        }
    }

    /// Makes a copy of this node, but with its copies relative to the given
    /// block and with fresh inputs.
    pub fn clone_in_block(
        self,
        block_from: BlockId,
        block_to: BlockId,
        inputs: &mut HashMap<InputId, NodeId>,
        a: &mut Arena,
    ) -> Self {
        match a[self] {
            Node::Copy(offset, block) if block == block_from => {
                Node::Copy(offset, block_to).insert_ideal(a)
            }
            Node::Copy(..) | Node::Const(_) => self,
            Node::Input(input) => {
                if let Some(&id) = inputs.get(&input) {
                    id
                } else {
                    let id = a.fresh_input();
                    inputs.insert(input, id);
                    id
                }
            }
            Node::Add(lhs, rhs) => {
                let lhs = lhs.clone_in_block(block_from, block_to, inputs, a);
                let rhs = rhs.clone_in_block(block_from, block_to, inputs, a);
                Node::Add(lhs, rhs).insert_ideal(a)
            }
            Node::Mul(lhs, rhs) => {
                let lhs = lhs.clone_in_block(block_from, block_to, inputs, a);
                let rhs = rhs.clone_in_block(block_from, block_to, inputs, a);
                Node::Mul(lhs, rhs).insert_ideal(a)
            }
        }
    }
}

impl Effect {
    /// Clones this effect, making its copies be relative to the given block and
    /// generating fresh inputs.
    pub fn clone_in_block(
        &self,
        block_from: BlockId,
        block_to: BlockId,
        inputs: &mut HashMap<InputId, NodeId>,
        a: &mut Arena,
    ) -> Self {
        match self {
            Effect::Output(values) => {
                let values = values
                    .iter()
                    .map(|value| value.clone_in_block(block_from, block_to, inputs, a))
                    .collect();
                Effect::Output(values)
            }
            Effect::Input(input) => {
                Effect::Input(input.clone_in_block(block_from, block_to, inputs, a))
            }
            &Effect::GuardShift(offset) => Effect::GuardShift(offset),
        }
    }
}
