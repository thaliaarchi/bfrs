use std::fmt::{self, Debug, Formatter};

use crate::{
    graph::{ArrayId, ByteId, Graph, NodeId},
    memory::{Memory, MemoryBuilder},
    node::{Array, Byte, Node},
    Ast,
};

/// A region of code, that tracks memory and effects. It currently corresponds
/// to a basic block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    /// The memory for this region.
    pub memory: Memory,
    /// A sequence of effects in this region.
    pub effects: Vec<Effect>,
    /// The number of inputs read in this region.
    pub(super) inputs: usize,
}

/// An observable effect.
#[derive(Clone, PartialEq, Eq)]
pub enum Effect {
    /// Printing a value. The node is a byte or an array.
    Output(NodeId),
    /// Reading from the user. The node is always `Node::Input`.
    Input(ByteId),
    /// Guarding that a shift can be performed by a certain amount.
    GuardShift(isize),
}

impl Region {
    /// Constructs a region from non-branching instructions.
    pub fn from_basic_block(insts: &[Ast], memory: &mut MemoryBuilder, g: &mut Graph) -> Self {
        memory.reset();
        let mut effects = Vec::new();
        let mut inputs = 0;
        for inst in insts {
            match inst {
                Ast::Right | Ast::Left => {
                    let amount = if inst == &Ast::Right { 1 } else { -1 };
                    if let Some(guard) = memory.shift(amount) {
                        effects.push(guard);
                    }
                }
                Ast::Inc | Ast::Dec => {
                    memory.add(if inst == &Ast::Inc { 1 } else { 255 });
                }
                Ast::Output => {
                    effects.push(Effect::Output(memory.compute_cell(g).as_node_id()));
                }
                Ast::Input => {
                    let input = Byte::Input { id: inputs }.insert(g);
                    inputs += 1;
                    effects.push(Effect::Input(input));
                    memory.set_cell(input);
                }
                Ast::Loop(_) => panic!("loops must be lowered separately"),
            }
        }
        Region {
            memory: memory.finish(g),
            effects,
            inputs,
        }
    }

    /// Concatenates two regions. Applies the operations of `other` to
    /// `self` and modifies `other`.
    pub fn concat(&mut self, other: &mut Self, g: &mut Graph) {
        self.effects.reserve(other.effects.len());
        for effect in &other.effects {
            let mut effect = effect.clone();
            match &mut effect {
                Effect::Output(value) => {
                    *value = self.rebase_node(*value, g);
                }
                Effect::Input(value) => {
                    let Byte::Input { id } = g[*value] else {
                        panic!("invalid node in input");
                    };
                    *value = Byte::Input {
                        id: id + self.inputs,
                    }
                    .insert(g);
                }
                Effect::GuardShift(offset) => {
                    *offset += self.memory.offset();
                    if !self.memory.guard_offset(*offset) {
                        continue;
                    }
                }
            }
            self.effects.push(effect);
        }
        self.join_outputs(g);

        for (_, cell) in other.memory.iter_mut() {
            *cell = self.rebase_byte(*cell, g);
        }
        self.memory.apply(&other.memory);
        self.inputs += other.inputs;
    }

    /// Replaces `Copy` and `Input` nodes in the node to be relative to this
    /// region.
    fn rebase_node(&mut self, node: NodeId, g: &mut Graph) -> NodeId {
        if let Some(id) = node.as_byte_id(g) {
            self.rebase_byte(id, g).as_node_id()
        } else if let Some(id) = node.as_array_id(g) {
            self.rebase_array(id, g).as_node_id()
        } else {
            unreachable!();
        }
    }

    /// Replaces `Copy` and `Input` nodes in the byte node to be relative to
    /// this region.
    fn rebase_byte(&mut self, node: ByteId, g: &mut Graph) -> ByteId {
        match g[node] {
            Byte::Copy(offset) => self.memory.compute_cell(self.memory.offset() + offset, g),
            Byte::Const(_) => node,
            Byte::Input { id } => Byte::Input {
                id: id + self.inputs,
            }
            .insert(g),
            Byte::Add(lhs, rhs) => {
                let lhs2 = self.rebase_byte(lhs, g);
                let rhs2 = self.rebase_byte(rhs, g);
                if lhs2 == lhs && rhs2 == rhs {
                    return node;
                }
                Byte::Add(lhs2, rhs2).idealize(g)
            }
            Byte::Mul(lhs, rhs) => {
                let lhs2 = self.rebase_byte(lhs, g);
                let rhs2 = self.rebase_byte(rhs, g);
                if lhs2 == lhs && rhs2 == rhs {
                    return node;
                }
                Byte::Mul(lhs2, rhs2).idealize(g)
            }
        }
    }

    /// Replaces `Copy` and `Input` nodes in the array node to be relative to
    /// this region.
    fn rebase_array(&mut self, node: ArrayId, g: &mut Graph) -> ArrayId {
        let mut array = g[node].clone();
        for e in &mut array.elements {
            *e = self.rebase_byte(*e, g);
        }
        array.insert(g)
    }

    /// Joins adjacent output effects into a single output of an array.
    pub fn join_outputs(&mut self, g: &mut Graph) {
        let mut i = 0;
        while i + 1 < self.effects.len() {
            if let Effect::Output(v1) = self.effects[i] {
                let j = self.effects[i + 1..]
                    .iter()
                    .position(|effect| !matches!(effect, Effect::Output(_)))
                    .map(|n| i + 1 + n)
                    .unwrap_or(self.effects.len());
                if j - i > 1 {
                    let mut array = match &g[v1] {
                        Node::Byte(_) => {
                            let mut elements = Vec::new();
                            elements.push(v1.as_byte_id(g).unwrap());
                            Array { elements }
                        }
                        Node::Array(array) => array.clone(),
                    };
                    for output in self.effects.drain(i + 1..j) {
                        let Effect::Output(v) = output else {
                            unreachable!();
                        };
                        match &g[v] {
                            Node::Byte(_) => array.elements.push(v.as_byte_id(g).unwrap()),
                            Node::Array(other) => array.elements.extend_from_slice(&other.elements),
                        }
                    }
                    self.effects[i] = Effect::Output(array.insert(g).as_node_id());
                }
            }
            i += 1;
        }
    }
}

impl Debug for Effect {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Effect::Output(v) => write!(f, "output {v:?}"),
            Effect::Input(v) => write!(f, "input {v:?}"),
            Effect::GuardShift(offset) => write!(f, "guard_shift {offset}"),
        }
    }
}
