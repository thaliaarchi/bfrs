use std::{
    collections::VecDeque,
    fmt::{self, Debug, Formatter},
};

use crate::{
    graph::{Graph, NodeId},
    node::Node,
    Ast,
};

// TODO:
// - Sort `Add` operands by offset.
// - Move guard_shift out of loops with no net shift. Peel the first iteration
//   if necessary.

#[derive(Clone, PartialEq, Eq)]
pub enum Ir {
    /// A basic block of non-branching instructions.
    BasicBlock(BasicBlock),
    /// Loop while some condition is true.
    Loop {
        /// Loop condition.
        condition: Condition,
        /// The contained blocks.
        body: Vec<Ir>,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Condition {
    /// Loop while the current cell is non-zero.
    WhileNonZero,
    /// Execute if the current cell is non-zero.
    IfNonZero,
    /// Loop a fixed number of times.
    Count(NodeId),
}

/// Abstract model of a sub-slice of memory and the effects in a basic block.
#[derive(Clone, PartialEq, Eq)]
pub struct BasicBlock {
    /// The sub-slice of memory, that is modified by this block.
    pub(crate) memory: VecDeque<NodeId>,
    /// The input sub-slice of memory, that is read by this block.
    pub(crate) memory_inputs: VecDeque<NodeId>,
    /// A sequence of effects in a basic block.
    pub(crate) effects: Vec<Effect>,
    /// The index in `memory` of the initial cell at the start of this basic
    /// block.
    pub(crate) origin_index: usize,
    /// The offset of the current cell relative to the initial cell of this
    /// basic block.
    pub(crate) offset: isize,
    /// The minimum shift left that has been guarded.
    pub(crate) guarded_left: isize,
    /// The maximum shift right that has been guarded.
    pub(crate) guarded_right: isize,
    /// The number of inputs read in this basic block.
    pub(crate) inputs: usize,
}

/// An observable effect.
#[derive(Clone, PartialEq, Eq)]
pub enum Effect {
    /// Printing a value.
    Output(NodeId),
    /// Reading from the user. The node is always `Node::Input`.
    Input(NodeId),
    /// Guarding that a shift can be performed to an offset.
    GuardShift(isize),
}

impl Ir {
    pub fn lower(ast: &[Ast], g: &mut Graph) -> Vec<Self> {
        let mut ir = vec![];
        for inst in ast {
            if let Ast::Loop(body) = inst {
                ir.push(Ir::Loop {
                    condition: Condition::WhileNonZero,
                    body: Ir::lower(body, g),
                });
            } else {
                if !matches!(ir.last(), Some(Ir::BasicBlock { .. })) {
                    ir.push(Ir::BasicBlock(BasicBlock::new()));
                }
                let Some(Ir::BasicBlock(bb)) = ir.last_mut() else {
                    unreachable!();
                };
                bb.apply(inst, g);
            }
        }
        ir
    }

    pub fn optimize_root(ir: &mut Vec<Ir>, g: &mut Graph) {
        let first_non_loop = ir
            .iter()
            .position(|block| !matches!(block, Ir::Loop { .. }))
            .unwrap_or(0);
        ir.drain(..first_non_loop);
        Ir::optimize_blocks(ir, g);
    }

    fn optimize_blocks(ir: &mut Vec<Ir>, g: &mut Graph) {
        ir.dedup_by(|block2, block1| match (block1, block2) {
            (
                Ir::Loop {
                    condition: Condition::WhileNonZero,
                    ..
                },
                Ir::Loop {
                    condition: Condition::WhileNonZero,
                    ..
                },
            ) => true,
            _ => false,
        });
        for block in ir.iter_mut() {
            block.optimize(g);
        }
        ir.dedup_by(|block2, block1| match (block1, block2) {
            (Ir::BasicBlock(block1), Ir::BasicBlock(block2)) => {
                block1.concat(block2, g);
                true
            }
            _ => false,
        });
    }

    /// Optimizes decrement loops and if-style loops.
    pub fn optimize(&mut self, g: &mut Graph) {
        if let Ir::Loop { condition, body } = self {
            if let [Ir::BasicBlock(bb)] = body.as_mut_slice() {
                if bb.offset == 0 {
                    if let Some(current) = bb.get(0) {
                        if let Node::Add(lhs, rhs) = g[current] {
                            if let (Node::Copy(0), &Node::Const(rhs)) = (&g[lhs], &g[rhs]) {
                                if let Some(iterations) = mod_inverse(rhs.wrapping_neg()) {
                                    let addend = Node::Mul(lhs, Node::Const(iterations).insert(g))
                                        .idealize(g);
                                    if !bb.memory.iter().zip(bb.min_offset()..).any(
                                        |(&cell, offset)| {
                                            cell.get(g).references_other(offset)
                                                || matches!(
                                                    g[cell],
                                                    Node::Input { .. } | Node::Mul(..)
                                                )
                                        },
                                    ) {
                                        if bb
                                            .effects
                                            .iter()
                                            .all(|effect| matches!(effect, Effect::GuardShift(_)))
                                        {
                                            *bb.get_mut(0).unwrap() = Node::Const(0).insert(g);
                                            for cell in &mut bb.memory {
                                                if let Node::Add(lhs, rhs) = g[*cell] {
                                                    *cell = Node::Add(
                                                        lhs,
                                                        Node::Mul(rhs, addend).idealize(g),
                                                    )
                                                    .idealize(g);
                                                }
                                            }
                                            let bb = body.drain(..).next().unwrap();
                                            *self = bb;
                                            return;
                                        }
                                    }
                                    *condition = Condition::Count(addend);
                                }
                            }
                        }
                    }
                }
            }
            Ir::optimize_blocks(body, g);
            match body.last() {
                Some(Ir::BasicBlock(last)) => {
                    if let Some(offset) = Ir::offset_root(body) {
                        if let Some(v) = last.get(offset) {
                            if g[v] == Node::Const(0) {
                                *condition = Condition::IfNonZero;
                            }
                        }
                    }
                }
                Some(Ir::Loop { .. }) => {
                    *condition = Condition::IfNonZero;
                }
                None => {}
            }
        }
    }

    fn offset_root(ir: &[Self]) -> Option<isize> {
        let mut offset = 0;
        for bb in ir {
            offset += bb.offset()?;
        }
        Some(offset)
    }

    fn offset(&self) -> Option<isize> {
        match self {
            Ir::BasicBlock(bb) => Some(bb.offset),
            Ir::Loop { body, .. } => {
                let mut offset = 0;
                for bb in body {
                    offset += bb.offset()?;
                }
                if offset != 0 {
                    return None;
                }
                Some(offset)
            }
        }
    }
}

impl BasicBlock {
    /// Constructs a basic block with no effects.
    pub fn new() -> Self {
        BasicBlock {
            memory: VecDeque::new(),
            memory_inputs: VecDeque::new(),
            effects: Vec::new(),
            origin_index: 0,
            offset: 0,
            guarded_left: 0,
            guarded_right: 0,
            inputs: 0,
        }
    }

    /// Applies an operation to the basic block.
    pub fn apply(&mut self, op: &Ast, g: &mut Graph) {
        match op {
            Ast::Right => {
                self.offset += 1;
                if self.offset > self.guarded_right {
                    self.guarded_right = self.offset;
                    self.effects.push(Effect::GuardShift(self.guarded_right));
                }
            }
            Ast::Left => {
                self.offset -= 1;
                if self.offset < self.guarded_left {
                    self.guarded_left = self.offset;
                    self.effects.push(Effect::GuardShift(self.guarded_left));
                }
            }
            Ast::Inc => {
                let cell = self.cell_mut(g);
                *cell = Node::Add(*cell, Node::Const(1).insert(g)).idealize(g)
            }
            Ast::Dec => {
                let cell = self.cell_mut(g);
                *cell = Node::Add(*cell, Node::Const(255).insert(g)).idealize(g)
            }
            Ast::Output => {
                let value = self.cell(self.offset, g);
                self.effects.push(Effect::Output(value));
            }
            Ast::Input => {
                let input = Node::Input { id: self.inputs }.insert(g);
                *self.cell_mut(g) = input;
                self.effects.push(Effect::Input(input));
                self.inputs += 1;
            }
            Ast::Loop(_) => panic!("loops must be lowered separately"),
        }
    }

    /// Concatenates two basic blocks. Applies the operations of `other` to
    /// `self`, draining `other`.
    pub fn concat(&mut self, other: &mut Self, g: &mut Graph) {
        self.effects.reserve(other.effects.len());
        for mut effect in other.effects.drain(..) {
            match &mut effect {
                Effect::Output(value) => {
                    *value = value.rebase(self, g);
                }
                Effect::Input(value) => {
                    let Node::Input { id } = g[*value] else {
                        panic!("invalid node in input");
                    };
                    *value = Node::Input {
                        id: id + self.inputs,
                    }
                    .insert(g);
                }
                Effect::GuardShift(offset) => {
                    *offset += self.offset;
                    if *offset < self.guarded_left {
                        self.guarded_left = *offset;
                    } else if *offset > self.guarded_right {
                        self.guarded_right = *offset;
                    } else {
                        continue;
                    }
                }
            }
            self.effects.push(effect);
        }
        for cell in &mut other.memory {
            *cell = cell.rebase(self, g);
        }
        let min_offset = self.offset + other.min_offset();
        self.reserve(min_offset, g);
        self.reserve((self.offset + other.max_offset() - 1).max(min_offset), g);
        for (i, cell) in other.memory.drain(..).enumerate() {
            self.memory[(self.origin_index as isize + min_offset) as usize + i] = cell;
        }
        self.guarded_left = self.guarded_left.min(self.offset + other.guarded_left);
        self.guarded_right = self.guarded_right.max(self.offset + other.guarded_right);
        self.offset += other.offset;
        self.inputs += other.inputs;
    }

    pub fn offset(&self) -> isize {
        self.offset
    }

    pub fn min_offset(&self) -> isize {
        -(self.origin_index as isize)
    }

    pub fn max_offset(&self) -> isize {
        self.min_offset() + self.memory.len() as isize
    }

    pub fn inputs(&self) -> usize {
        self.inputs
    }

    fn reserve(&mut self, offset: isize, g: &mut Graph) {
        if offset < self.min_offset() {
            let n = (self.min_offset() - offset) as usize;
            self.memory.reserve(n);
            self.memory_inputs.reserve(n);
            for i in (offset..self.min_offset()).rev() {
                let copy = Node::Copy(i).insert(g);
                self.memory.push_front(copy);
                self.memory_inputs.push_front(copy);
            }
            self.origin_index += n;
        } else if offset >= self.max_offset() {
            let n = (offset - self.max_offset()) as usize + 1;
            self.memory.reserve(n);
            for i in self.max_offset()..=offset {
                let copy = Node::Copy(i).insert(g);
                self.memory.push_back(copy);
                self.memory_inputs.push_back(copy);
            }
        }
    }

    fn get(&self, offset: isize) -> Option<NodeId> {
        usize::try_from(self.origin_index as isize + offset)
            .ok()
            .and_then(|i| self.memory.get(i).copied())
    }

    fn get_mut(&mut self, offset: isize) -> Option<&mut NodeId> {
        usize::try_from(self.origin_index as isize + offset)
            .ok()
            .and_then(|i| self.memory.get_mut(i))
    }

    pub(crate) fn cell(&mut self, offset: isize, g: &mut Graph) -> NodeId {
        self.reserve(offset, g);
        self.memory[(self.origin_index as isize + offset) as usize]
    }

    fn cell_mut(&mut self, g: &mut Graph) -> &mut NodeId {
        self.reserve(self.offset, g);
        &mut self.memory[(self.origin_index as isize + self.offset) as usize]
    }
}

impl Debug for Ir {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            Ir::BasicBlock(bb) => Debug::fmt(bb, f),
            Ir::Loop { condition, body } => {
                write!(f, "Loop({condition:?}) ")?;
                f.debug_list().entries(body).finish()
            }
        }
    }
}

impl Debug for BasicBlock {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        struct Memory<'a>(&'a BasicBlock);
        impl Debug for Memory<'_> {
            fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
                f.debug_map()
                    .entries(
                        (self.0.min_offset()..)
                            .zip(self.0.memory.iter())
                            .map(|(offset, &node)| (offset, node)),
                    )
                    .finish()
            }
        }
        f.debug_struct("BasicBlock")
            .field("memory", &Memory(self))
            .field("effects", &self.effects)
            .field("offset", &self.offset)
            .field("guarded", &(self.guarded_left..=self.guarded_right))
            .field("inputs", &self.inputs)
            .finish()
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

/// Computes the multiplicative inverse of a number (mod 256).
fn mod_inverse(value: u8) -> Option<u8> {
    static INVERSES: [u8; 128] = [
        /*1*/ 1, /*3*/ 171, /*5*/ 205, /*7*/ 183, /*9*/ 57,
        /*11*/ 163, /*13*/ 197, /*15*/ 239, /*17*/ 241, /*19*/ 27,
        /*21*/ 61, /*23*/ 167, /*25*/ 41, /*27*/ 19, /*29*/ 53,
        /*31*/ 223, /*33*/ 225, /*35*/ 139, /*37*/ 173, /*39*/ 151,
        /*41*/ 25, /*43*/ 131, /*45*/ 165, /*47*/ 207, /*49*/ 209,
        /*51*/ 251, /*53*/ 29, /*55*/ 135, /*57*/ 9, /*59*/ 243,
        /*61*/ 21, /*63*/ 191, /*65*/ 193, /*67*/ 107, /*69*/ 141,
        /*71*/ 119, /*73*/ 249, /*75*/ 99, /*77*/ 133, /*79*/ 175,
        /*81*/ 177, /*83*/ 219, /*85*/ 253, /*87*/ 103, /*89*/ 233,
        /*91*/ 211, /*93*/ 245, /*95*/ 159, /*97*/ 161, /*99*/ 75,
        /*101*/ 109, /*103*/ 87, /*105*/ 217, /*107*/ 67, /*109*/ 101,
        /*111*/ 143, /*113*/ 145, /*115*/ 187, /*117*/ 221, /*119*/ 71,
        /*121*/ 201, /*123*/ 179, /*125*/ 213, /*127*/ 127, /*129*/ 129,
        /*131*/ 43, /*133*/ 77, /*135*/ 55, /*137*/ 185, /*139*/ 35,
        /*141*/ 69, /*143*/ 111, /*145*/ 113, /*147*/ 155, /*149*/ 189,
        /*151*/ 39, /*153*/ 169, /*155*/ 147, /*157*/ 181, /*159*/ 95,
        /*161*/ 97, /*163*/ 11, /*165*/ 45, /*167*/ 23, /*169*/ 153,
        /*171*/ 3, /*173*/ 37, /*175*/ 79, /*177*/ 81, /*179*/ 123,
        /*181*/ 157, /*183*/ 7, /*185*/ 137, /*187*/ 115, /*189*/ 149,
        /*191*/ 63, /*193*/ 65, /*195*/ 235, /*197*/ 13, /*199*/ 247,
        /*201*/ 121, /*203*/ 227, /*205*/ 5, /*207*/ 47, /*209*/ 49,
        /*211*/ 91, /*213*/ 125, /*215*/ 231, /*217*/ 105, /*219*/ 83,
        /*221*/ 117, /*223*/ 31, /*225*/ 33, /*227*/ 203, /*229*/ 237,
        /*231*/ 215, /*233*/ 89, /*235*/ 195, /*237*/ 229, /*239*/ 15,
        /*241*/ 17, /*243*/ 59, /*245*/ 93, /*247*/ 199, /*249*/ 73,
        /*251*/ 51, /*253*/ 85, /*255*/ 255,
    ];
    if value % 2 == 1 {
        Some(INVERSES[value as usize >> 1])
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use crate::{graph::Graph, ir::Ir, Ast};

    #[test]
    fn concat() {
        let src1 = "<+>,-.>";
        let src2 = ",<-";
        let g = &mut Graph::new();
        let ir1 = Ir::lower(&Ast::parse(src1.as_bytes()).unwrap(), g);
        let ir2 = Ir::lower(&Ast::parse(src2.as_bytes()).unwrap(), g);
        let (mut bb1, mut bb2) = match (&*ir1, &*ir2) {
            ([Ir::BasicBlock(bb1)], [Ir::BasicBlock(bb2)]) => (bb1.clone(), bb2.clone()),
            _ => panic!("not single basic blocks: {ir1:?}, {ir2:?}"),
        };
        bb1.concat(&mut bb2, g);
        let expect = "
            guard_shift -1
            in0 = input
            output in0 - 1
            guard_shift 1
            in1 = input
            @-1 = @-1 + 1
            @0 = in0 - 2
            @1 = in1
        ";
        assert!(bb1.compare_pretty(expect, g));
    }
}
