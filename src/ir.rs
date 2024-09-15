use std::{
    collections::VecDeque,
    fmt::{self, Debug, Formatter},
    ops::RangeInclusive,
};

use crate::{
    graph::{ByteId, Graph, NodeId},
    node::{Array, Byte, Node},
    Ast,
};

// TODO:
// - Transforming a loop with shifts to its closed form is unsound, when those
//   shifts have not already been guarded.
// - Check for guaranteed zero recursively instead of iteratively.
// - Add infinite loop condition.
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
    Count(ByteId),
}

/// Abstract model of a sub-slice of memory and the effects in a basic block.
#[derive(Clone, PartialEq, Eq)]
pub struct BasicBlock {
    /// The sub-slice of memory, that is modified by this block.
    pub(crate) memory: VecDeque<Option<ByteId>>,
    /// The input sub-slice of memory, that is read by this block.
    pub(crate) memory_copies: VecDeque<Option<ByteId>>,
    /// A sequence of effects in a basic block.
    pub(crate) effects: Vec<Effect>,
    /// The offset of the first cell in `memory` of this basic block.
    pub(crate) min_offset: isize,
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
    Input(ByteId),
    /// Guarding that a shift can be performed to an offset.
    GuardShift(isize),
}

/// A builder for a basic block.
#[derive(Debug)]
struct BasicBlockBuilder<'a> {
    g: &'a mut Graph,
    memory: &'a mut VecDeque<(Option<ByteId>, u8)>,
    effects: Vec<Effect>,
    min_offset: isize,
    offset: isize,
    guarded_left: isize,
    guarded_right: isize,
    inputs: usize,
}

impl Ir {
    pub fn lower(mut ast: &[Ast], g: &mut Graph) -> Vec<Self> {
        let mut memory = VecDeque::new();
        let mut ir = vec![];
        while let Some((inst, rest)) = ast.split_first() {
            if let Ast::Loop(body) = inst {
                ir.push(Ir::Loop {
                    condition: Condition::WhileNonZero,
                    body: Ir::lower(body, g),
                });
                ast = rest;
            } else {
                let i = ast
                    .iter()
                    .position(|inst| matches!(inst, Ast::Loop(_)))
                    .unwrap_or(ast.len());
                let (linear_insts, rest) = ast.split_at(i);
                memory.clear();
                let mut b = BasicBlockBuilder::new(g, &mut memory);
                for inst in linear_insts {
                    b.apply(inst);
                }
                ir.push(Ir::BasicBlock(b.finish()));
                ast = rest;
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
        if let Ir::BasicBlock(bb) = self {
            bb.combine_outputs(g);
        } else if let Ir::Loop { condition, body } = self {
            Ir::optimize_blocks(body, g);
            if let [Ir::BasicBlock(bb)] = body.as_mut_slice() {
                if bb.offset == 0 {
                    if let Some(current) = bb.get(0) {
                        if let Byte::Add(lhs, rhs) = g[current] {
                            if let (Byte::Copy(0), &Byte::Const(rhs)) = (&g[lhs], &g[rhs]) {
                                if let Some(iterations) = mod_inverse(rhs.wrapping_neg()) {
                                    let addend = Byte::Mul(lhs, Byte::Const(iterations).insert(g))
                                        .idealize(g);
                                    if !bb.memory.iter().zip(bb.min_offset..).any(
                                        |(&cell, offset)| {
                                            matches!(cell, Some(cell)
                                                if cell.get(g).references_other(offset)
                                                    || matches!(
                                                        g[cell],
                                                        Byte::Input { .. } | Byte::Mul(..)
                                                    )
                                            )
                                        },
                                    ) {
                                        if bb
                                            .effects
                                            .iter()
                                            .all(|effect| matches!(effect, Effect::GuardShift(_)))
                                        {
                                            *bb.get_mut(0) = Some(Byte::Const(0).insert(g));
                                            for cell in &mut bb.memory {
                                                if let Some(cell) = cell.as_mut() {
                                                    if let Byte::Add(lhs, rhs) = g[*cell] {
                                                        *cell = Byte::Add(
                                                            lhs,
                                                            Byte::Mul(rhs, addend).idealize(g),
                                                        )
                                                        .idealize(g);
                                                    }
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
            let mut guaranteed_zero = false;
            let mut offset = 0;
            for block in body.iter().rev() {
                match block {
                    Ir::BasicBlock(bb) => {
                        if let Some(v) = bb.get(bb.offset()) {
                            match g[v] {
                                Byte::Const(0) => {
                                    guaranteed_zero = true;
                                    break;
                                }
                                Byte::Copy(0) => {}
                                _ => {
                                    guaranteed_zero = false;
                                    break;
                                }
                            }
                        }
                        offset += bb.offset();
                    }
                    Ir::Loop { .. } => {
                        guaranteed_zero = offset == 0;
                        break;
                    }
                }
            }
            if guaranteed_zero {
                *condition = Condition::IfNonZero;
            }
        }
    }
}

impl<'a> BasicBlockBuilder<'a> {
    fn new(g: &'a mut Graph, memory: &'a mut VecDeque<(Option<ByteId>, u8)>) -> Self {
        BasicBlockBuilder {
            g,
            memory,
            effects: Vec::new(),
            min_offset: 0,
            offset: 0,
            guarded_left: 0,
            guarded_right: 0,
            inputs: 0,
        }
    }

    /// Applies an operation to the basic block.
    fn apply(&mut self, inst: &Ast) {
        match inst {
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
            Ast::Inc | Ast::Dec => {
                let (_, addend) = self.get_cell();
                *addend = addend.wrapping_add(if inst == &Ast::Inc { 1 } else { 255 });
            }
            Ast::Output => {
                let (base, addend) = *self.get_cell();
                let base = base.unwrap_or_else(|| Byte::Copy(self.offset).insert(self.g));
                let value = if addend != 0 {
                    Byte::Add(base, Byte::Const(addend).insert(self.g)).insert(self.g)
                } else {
                    base
                };
                self.effects.push(Effect::Output(value.as_node_id()));
            }
            Ast::Input => {
                let input = Byte::Input { id: self.inputs }.insert(self.g);
                *self.get_cell() = (Some(input), 0);
                self.effects.push(Effect::Input(input));
                self.inputs += 1;
            }
            Ast::Loop(_) => panic!("loops must be lowered separately"),
        }
    }

    fn get_cell(&mut self) -> &mut (Option<ByteId>, u8) {
        if self.memory.is_empty() {
            self.memory.push_back((None, 0));
            self.min_offset = self.offset;
        } else if self.offset < self.min_offset {
            for _ in 0..(self.min_offset - self.offset) {
                self.memory.push_front((None, 0));
            }
            self.min_offset = self.offset;
        } else if self.offset >= self.min_offset + self.memory.len() as isize {
            self.memory
                .resize((self.offset - self.min_offset + 1) as usize, (None, 0));
        }
        &mut self.memory[(self.offset - self.min_offset) as usize]
    }

    fn finish(self) -> BasicBlock {
        let g = self.g;
        let mut memory = VecDeque::with_capacity(self.memory.len());
        let mut memory_copies = VecDeque::with_capacity(self.memory.len());
        for (&(base, addend), offset) in self.memory.iter().zip(self.min_offset..) {
            let (base, copy) = match base {
                Some(base) => (Some(base), None),
                None if addend != 0 => {
                    let copy = Byte::Copy(offset).insert(g);
                    (Some(copy), Some(copy))
                }
                None => (None, None),
            };
            let value = base.map(|base| {
                if addend != 0 {
                    Byte::Add(base, Byte::Const(addend).insert(g)).insert(g)
                } else {
                    base
                }
            });
            memory.push_back(value);
            memory_copies.push_back(copy);
        }
        BasicBlock {
            memory,
            memory_copies,
            effects: self.effects,
            min_offset: self.min_offset,
            offset: self.offset,
            guarded_left: self.guarded_left,
            guarded_right: self.guarded_right,
            inputs: self.inputs,
        }
    }
}

impl BasicBlock {
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
                    let Byte::Input { id } = g[*value] else {
                        panic!("invalid node in input");
                    };
                    *value = Byte::Input {
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
        self.combine_outputs(g);
        for cell in &mut other.memory {
            if let Some(cell) = cell {
                *cell = cell.rebase(self, g);
            }
        }
        let min_offset = self.offset + other.min_offset;
        self.reserve(min_offset..=(self.offset + other.max_offset() - 1).max(min_offset));
        for (i, cell) in other.memory.drain(..).enumerate() {
            if let Some(cell) = cell {
                self.memory[(min_offset - self.min_offset) as usize + i] = Some(cell);
            }
        }
        self.guarded_left = self.guarded_left.min(self.offset + other.guarded_left);
        self.guarded_right = self.guarded_right.max(self.offset + other.guarded_right);
        self.offset += other.offset;
        self.inputs += other.inputs;
    }

    fn combine_outputs(&mut self, g: &mut Graph) {
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

    pub fn offset(&self) -> isize {
        self.offset
    }

    pub fn min_offset(&self) -> isize {
        self.min_offset
    }

    pub fn max_offset(&self) -> isize {
        self.min_offset + self.memory.len() as isize
    }

    pub fn inputs(&self) -> usize {
        self.inputs
    }

    fn reserve(&mut self, offsets: RangeInclusive<isize>) {
        debug_assert_eq!(self.memory.len(), self.memory_copies.len());
        if self.memory.is_empty() {
            let n = (offsets.end() - offsets.start() + 1) as usize;
            self.memory.resize(n, None);
            self.memory_copies.resize(n, None);
            self.min_offset = *offsets.start();
            return;
        }
        let min_offset = self.min_offset.min(*offsets.start());
        let max_offset = self.max_offset().max(*offsets.end());
        let len = (max_offset - min_offset + 1) as usize;
        let additional = len - self.memory.len();
        self.memory.reserve(additional);
        self.memory_copies.reserve(additional);
        for _ in 0..(self.min_offset - min_offset) as usize {
            self.memory.push_front(None);
            self.memory_copies.push_front(None);
        }
        self.memory.resize(len, None);
        self.memory_copies.resize(len, None);
        self.min_offset = min_offset;
    }

    fn get(&self, offset: isize) -> Option<ByteId> {
        self.memory
            .get(usize::try_from(offset - self.min_offset).ok()?)
            .copied()
            .flatten()
    }

    fn get_mut(&mut self, offset: isize) -> &mut Option<ByteId> {
        self.reserve(offset..=offset);
        &mut self.memory[(offset - self.min_offset) as usize]
    }

    pub(crate) fn get_or_copy(&mut self, offset: isize, g: &mut Graph) -> &mut ByteId {
        self.reserve(offset..=offset);
        let i = (offset - self.min_offset) as usize;
        match &mut self.memory[i] {
            Some(cell) => cell,
            cell @ None => {
                debug_assert!(self.memory_copies[i].is_none());
                let copy = Byte::Copy(offset).insert(g);
                *cell = Some(copy);
                self.memory_copies[i] = Some(copy);
                cell.as_mut().unwrap()
            }
        }
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
                        (self.0.min_offset..)
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

    fn assert_node_count(src: &str, nodes: usize) {
        let mut g = Graph::new();
        Ir::lower(&Ast::parse(src.as_bytes()).unwrap(), &mut g);
        assert_eq!(g.len(), nodes, "{src:?} => {g:?}");
    }

    #[test]
    fn lazy_node_construction() {
        assert_node_count(">>>", 0);
        assert_node_count(">>>+", 3);
        assert_node_count(">>>-", 3);
        assert_node_count(">>>,", 1);
        assert_node_count(">>>.", 1);
        assert_node_count(">+<>++++-><-+-+>><<-+-+++", 3);
    }

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
