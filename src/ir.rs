use std::{
    collections::VecDeque,
    fmt::{self, Debug, Formatter},
    mem,
};

use crate::{Ast, Value};

// TODO:
// - Concatenate flattened loops.
// - Sort `Add` operands by offset.

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
    Count(Value),
}

/// Abstract model of a sub-slice of memory and the effects in a basic block.
#[derive(Clone, PartialEq, Eq)]
pub struct BasicBlock {
    /// A sub-slice of the full memory.
    pub(crate) memory: VecDeque<Value>,
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
    Output(Value),
    /// Reading from the user.
    Input { id: usize },
    /// Guarding that a shift can be performed to an offset.
    GuardShift(isize),
}

impl Ir {
    pub fn lower(ast: &[Ast]) -> Vec<Self> {
        let mut ir = vec![];
        for inst in ast {
            if let Ast::Loop(body) = inst {
                ir.push(Ir::Loop {
                    condition: Condition::WhileNonZero,
                    body: Ir::lower(body),
                });
            } else {
                if !matches!(ir.last(), Some(Ir::BasicBlock { .. })) {
                    ir.push(Ir::BasicBlock(BasicBlock::new()));
                }
                let Some(Ir::BasicBlock(bb)) = ir.last_mut() else {
                    unreachable!();
                };
                bb.apply(inst);
            }
        }
        ir
    }

    pub fn optimize_root(ir: &mut [Ir]) {
        for node in ir {
            node.optimize();
        }
    }

    /// Optimizes decrement loops and if-style loops.
    pub fn optimize(&mut self) {
        if let Ir::Loop { condition, body } = self {
            if let [Ir::BasicBlock(bb)] = body.as_mut_slice() {
                if bb.offset == 0 {
                    if let Some(Value::Add(lhs, rhs)) = bb.get(0) {
                        if let (Value::Copy(0), &Value::Const(rhs)) = (lhs.as_ref(), rhs.as_ref()) {
                            if let Some(iterations) = mod_inverse(rhs.wrapping_neg()) {
                                let mut addend = Value::Copy(0);
                                if iterations != 1 {
                                    addend = Value::mul(
                                        Box::new(addend),
                                        Box::new(Value::Const(iterations)),
                                    );
                                }
                                if !bb
                                    .memory
                                    .iter()
                                    .zip(bb.min_offset()..)
                                    .any(|(cell, offset)| {
                                        cell.references_other(offset)
                                            || matches!(cell, Value::Input { .. } | Value::Mul(..))
                                    })
                                {
                                    if bb
                                        .effects
                                        .iter()
                                        .all(|effect| matches!(effect, Effect::GuardShift(_)))
                                    {
                                        *bb.get_mut(0).unwrap() = Value::Const(0);
                                        for cell in &mut bb.memory {
                                            if let Value::Add(_, rhs) = cell {
                                                let rhs1 = mem::take(rhs.as_mut());
                                                *rhs = Box::new(Value::mul(
                                                    Box::new(rhs1),
                                                    Box::new(addend.clone()),
                                                ));
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
            if let Some(Ir::BasicBlock(last)) = body.last() {
                if let Some(offset) = Ir::offset_root(body) {
                    if last.get(offset) == Some(&Value::Const(0)) {
                        *condition = Condition::IfNonZero;
                        return;
                    }
                }
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
            effects: Vec::new(),
            origin_index: 0,
            offset: 0,
            guarded_left: 0,
            guarded_right: 0,
            inputs: 0,
        }
    }

    /// Applies an operation to the basic block.
    pub fn apply(&mut self, op: &Ast) {
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
            Ast::Inc => self.cell_mut().add_const(1),
            Ast::Dec => self.cell_mut().add_const(255),
            Ast::Output => {
                let value = self.cell_mut().clone();
                self.effects.push(Effect::Output(value));
            }
            Ast::Input => {
                *self.cell_mut() = Value::Input { id: self.inputs };
                self.effects.push(Effect::Input { id: self.inputs });
                self.inputs += 1;
            }
            Ast::Loop(_) => panic!("loops must be lowered separately"),
        }
    }

    /// Concatenates two basic blocks. Applies the operations of `other` to
    /// `self`, draining `other`.
    pub fn concat(&mut self, other: &mut Self) {
        self.effects.reserve(other.effects.len());
        for mut effect in other.effects.drain(..) {
            match &mut effect {
                Effect::Output(value) => {
                    *value = mem::take(value).rebase(self);
                }
                Effect::Input { id } => *id += self.inputs,
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
            *cell = mem::take(cell).rebase(self);
        }
        let min_offset = self.offset + other.min_offset();
        self.reserve(min_offset);
        self.reserve((self.offset + other.max_offset() - 1).max(min_offset));
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

    fn reserve(&mut self, offset: isize) {
        if offset < self.min_offset() {
            let n = (self.min_offset() - offset) as usize;
            self.memory.reserve(n);
            for i in (offset..self.min_offset()).rev() {
                self.memory.push_front(Value::Copy(i));
            }
            self.origin_index += n;
        } else if offset >= self.max_offset() {
            let n = (offset - self.max_offset()) as usize + 1;
            self.memory.reserve(n);
            for i in self.max_offset()..=offset {
                self.memory.push_back(Value::Copy(i));
            }
        }
    }

    fn get(&self, offset: isize) -> Option<&Value> {
        usize::try_from(self.origin_index as isize + offset)
            .ok()
            .and_then(|i| self.memory.get(i))
    }

    fn get_mut(&mut self, offset: isize) -> Option<&mut Value> {
        usize::try_from(self.origin_index as isize + offset)
            .ok()
            .and_then(|i| self.memory.get_mut(i))
    }

    fn cell_mut(&mut self) -> &mut Value {
        self.reserve(self.offset);
        &mut self.memory[(self.origin_index as isize + self.offset) as usize]
    }

    pub(crate) fn cell_copy(&self, offset: isize) -> Value {
        let index = self.origin_index as isize - offset;
        if 0 <= index && index < self.memory.len() as isize {
            self.memory[index as usize].clone()
        } else {
            Value::Copy(offset)
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
                        (self.0.min_offset()..)
                            .map(Value::Copy)
                            .zip(self.0.memory.iter())
                            .filter(|(k, v)| k != *v),
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
            Effect::Input { id } => write!(f, "input {id}"),
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
    use std::collections::VecDeque;

    use crate::{
        ir::{BasicBlock, Ir},
        Ast, Value,
    };

    #[test]
    fn apply_ops() {
        let mut bb = BasicBlock::new();
        bb.apply(&Ast::Inc);
        bb.apply(&Ast::Inc);
        bb.apply(&Ast::Right);
        bb.apply(&Ast::Left);
        bb.apply(&Ast::Left);
        bb.apply(&Ast::Dec);
        bb.apply(&Ast::Right);
        bb.apply(&Ast::Output);
        bb.apply(&Ast::Input);
        bb.apply(&Ast::Right);
        bb.apply(&Ast::Right);
        let expect = "
            guard_shift 1
            guard_shift -1
            output %0 + 2
            input 0
            guard_shift 2
            %-1 = %-1 + 255
            %0 = in0
            offset 2
        ";
        assert!(bb.compare_pretty(expect));
    }

    #[test]
    fn lower() {
        // Excerpt from https://www.brainfuck.org/collatz.b
        let src = b"[-[<->-]+[<<<<]]<[>+<-]";
        let ast = Ast::parse(src).unwrap();
        let ir = Ir::lower(&ast);
        let expect = "
            while %0 != 0 {
                {
                    %0 = %0 + 255
                }
                while %0 != 0 {
                    guard_shift -1
                    %-1 = %-1 + 255
                    %0 = %0 + 255
                }
                {
                    %0 = %0 + 1
                }
                while %0 != 0 {
                    guard_shift -1
                    guard_shift -2
                    guard_shift -3
                    guard_shift -4
                    offset -4
                }
            }
            {
                guard_shift -1
                offset -1
            }
            while %0 != 0 {
                guard_shift 1
                %0 = %0 + 255
                %1 = %1 + 1
            }
        ";
        assert!(Ir::compare_pretty_root(&ir, expect));
    }

    #[test]
    fn concat() {
        let src1 = "<+>,-.>";
        let src2 = ",<-";
        let ir1 = Ir::lower(&Ast::parse(src1.as_bytes()).unwrap());
        let ir2 = Ir::lower(&Ast::parse(src2.as_bytes()).unwrap());
        let (mut bb1, mut bb2) = match (&*ir1, &*ir2) {
            ([Ir::BasicBlock(bb1)], [Ir::BasicBlock(bb2)]) => (bb1.clone(), bb2.clone()),
            _ => panic!("not single basic blocks: {ir1:?}, {ir2:?}"),
        };
        bb1.concat(&mut bb2);
        let ir12 = Ir::lower(&Ast::parse((src1.to_owned() + src2).as_bytes()).unwrap());
        assert_eq!(vec![Ir::BasicBlock(bb1)], ir12);
    }

    #[test]
    fn optimize() {
        let src = b"[-]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expect = "
            %0 = 0
        ";
        assert!(Ir::compare_pretty_root(&ir, expect));

        let src = b"[->+<]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expect = "
            guard_shift 1
            %0 = 0
            %1 = %1 + %0
        ";
        assert!(Ir::compare_pretty_root(&ir, expect));

        let src = b"[->+++<]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expect = "
            guard_shift 1
            %0 = 0
            %1 = %1 + %0 * 3
        ";
        assert!(Ir::compare_pretty_root(&ir, expect));

        let src = b"[.-]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expect = "
            repeat %0 times {
                output %0
                %0 = %0 + 255
            }
        ";
        assert!(Ir::compare_pretty_root(&ir, expect));
    }

    #[test]
    fn closed_form_loops() {
        let src = b"[->-->+++<<]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expect = "
            guard_shift 1
            guard_shift 2
            %0 = 0
            %1 = %1 + %0 * 254
            %2 = %2 + %0 * 3
        ";
        assert!(Ir::compare_pretty_root(&ir, expect));

        let src = b"[--->+>++>->--<<<<]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expect = "
            guard_shift 1
            guard_shift 2
            guard_shift 3
            guard_shift 4
            %0 = 0
            %1 = %1 + %0 * 171
            %2 = %2 + %0 * 86
            %3 = %3 + %0 * 85
            %4 = %4 + %0 * 170
        ";
        assert!(Ir::compare_pretty_root(&ir, expect));

        let src = b"[+++++++++++++++.>++<]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expect = "
            repeat %0 * 17 times {
                output %0 + 15
                guard_shift 1
                %0 = %0 + 15
                %1 = %1 + 2
            }
        ";
        assert!(Ir::compare_pretty_root(&ir, expect));
    }

    #[test]
    fn reserve() {
        let mut bb = BasicBlock::new();
        bb.reserve(1);
        bb.reserve(-2);
        bb.reserve(2);
        bb.reserve(-3);
        let expect = BasicBlock {
            memory: VecDeque::from([
                Value::Copy(-3),
                Value::Copy(-2),
                Value::Copy(-1),
                Value::Copy(0),
                Value::Copy(1),
                Value::Copy(2),
            ]),
            effects: vec![],
            origin_index: 3,
            offset: 0,
            guarded_left: 0,
            guarded_right: 0,
            inputs: 0,
        };
        assert_eq!(bb, expect);
    }
}
