use crate::{
    graph::{Graph, NodeId},
    memory::{Memory, MemoryBuilder},
    node::Node,
    Ast,
};

/// A region of code, that tracks memory and effects. It currently corresponds
/// to a basic block.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Region {
    /// The memory for this region.
    pub memory: Memory,
    /// A sequence of effects in this region.
    pub effects: Vec<Effect>,
    /// The number of inputs read in this region.
    pub(super) inputs: usize,
}

/// An observable effect.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Effect {
    /// Printing a value. The node is a byte or an array.
    Output(NodeId),
    /// Reading from the user. The node is always `Node::Input`.
    Input(NodeId),
    /// Guarding that a shift can be performed by a certain amount.
    GuardShift(isize),
}

impl Region {
    /// Constructs a region from non-branching instructions.
    pub fn from_basic_block(insts: &[Ast], memory: &mut MemoryBuilder, g: &Graph) -> Self {
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
                    effects.push(Effect::Output(memory.compute_cell(g)));
                }
                Ast::Input => {
                    let input = Node::Input { id: inputs }.insert(g);
                    inputs += 1;
                    effects.push(Effect::Input(input));
                    memory.set_cell(input);
                }
                Ast::Loop(_) => panic!("loops must be lowered separately"),
                Ast::Root(_) => panic!("unexpected root"),
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
    pub fn concat(&mut self, other: &Self, g: &Graph) {
        self.effects.reserve(other.effects.len());
        for effect in &other.effects {
            let mut effect = effect.clone();
            match &mut effect {
                Effect::Output(value) => {
                    *value = self.rebase(*value, g);
                }
                Effect::Input(value) => {
                    let Node::Input { id } = *g.get(*value) else {
                        panic!("invalid node in input");
                    };
                    *value = Node::Input {
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

        let mut other_memory = other.memory.clone();
        for (_, cell) in other_memory.iter_mut() {
            *cell = self.rebase(*cell, g);
        }
        self.memory.apply(&other_memory);
        self.inputs += other.inputs;
    }

    /// Replaces `Copy` and `Input` nodes in the node to be relative to this
    /// region.
    fn rebase(&mut self, node: NodeId, g: &Graph) -> NodeId {
        match *g.get(node) {
            Node::Root { .. } | Node::BasicBlock(_) | Node::Loop { .. } => {
                panic!("unexpected control node")
            }
            Node::Copy(offset) => self.memory.compute_cell(self.memory.offset() + offset, g),
            Node::Const(_) => node,
            Node::Input { id } => Node::Input {
                id: id + self.inputs,
            }
            .insert(g),
            Node::Add(lhs, rhs) => {
                let lhs2 = self.rebase(lhs, g);
                let rhs2 = self.rebase(rhs, g);
                if lhs2 == lhs && rhs2 == rhs {
                    return node;
                }
                Node::Add(lhs2, rhs2).idealize(g)
            }
            Node::Mul(lhs, rhs) => {
                let lhs2 = self.rebase(lhs, g);
                let rhs2 = self.rebase(rhs, g);
                if lhs2 == lhs && rhs2 == rhs {
                    return node;
                }
                Node::Mul(lhs2, rhs2).idealize(g)
            }
            Node::Array(ref elements) => {
                let mut elements2 = elements.clone();
                for e in &mut elements2 {
                    *e = self.rebase(*e, g);
                }
                Node::Array(elements2).insert(g)
            }
        }
    }

    /// Joins adjacent output effects into a single output of an array.
    pub fn join_outputs(&mut self, g: &Graph) {
        let mut i = 0;
        while i + 1 < self.effects.len() {
            if let Effect::Output(v1) = self.effects[i] {
                let j = self.effects[i + 1..]
                    .iter()
                    .position(|effect| !matches!(effect, Effect::Output(_)))
                    .map(|n| i + 1 + n)
                    .unwrap_or(self.effects.len());
                if j - i > 1 {
                    let mut elements = match &*g.get(v1) {
                        Node::Array(elements) => elements.clone(),
                        _ => vec![v1],
                    };
                    for output in self.effects.drain(i + 1..j) {
                        let Effect::Output(v) = output else {
                            unreachable!();
                        };
                        match &*g.get(v) {
                            Node::Array(other) => elements.extend_from_slice(&other),
                            _ => elements.push(v),
                        }
                    }
                    self.effects[i] = Effect::Output(Node::Array(elements).insert(g));
                }
            }
            i += 1;
        }
    }
}
