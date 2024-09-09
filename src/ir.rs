use std::{collections::VecDeque, mem};

use crate::{Ast, Value};

// TODO:
// - Concatenate flattened loops.
// - Sort `Add` operands by offset.

#[derive(Clone, Debug, PartialEq, Eq)]
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
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicBlock {
    /// A sub-slice of the full memory.
    memory: VecDeque<Value>,
    /// A sequence of effects in a basic block.
    effects: Vec<Effect>,
    /// The index in `memory` of the initial cell at the start of this basic
    /// block.
    origin_index: usize,
    /// The offset of the current cell relative to the initial cell of this
    /// basic block.
    offset: isize,
    /// The minimum shift left that has been guarded.
    guarded_left: isize,
    /// The maximum shift right that has been guarded.
    guarded_right: isize,
    /// The number of inputs read in this basic block.
    inputs: usize,
}

/// An observable effect.
#[derive(Clone, Debug, PartialEq, Eq)]
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
                        if **lhs == Value::Copy(0) && **rhs == Value::Const(255) {
                            if !bb
                                .memory
                                .iter()
                                .zip(bb.min_offset()..)
                                .any(|(cell, offset)| {
                                    cell.references_other(offset) || matches!(cell, Value::Mul(..))
                                })
                            {
                                if bb
                                    .effects
                                    .iter()
                                    .all(|effect| matches!(effect, Effect::GuardShift(_)))
                                {
                                    *bb.get_mut(0).unwrap() = Value::Const(0);
                                    for cell in &mut bb.memory {
                                        match cell {
                                            Value::Copy(_) | Value::Const(_) => {}
                                            Value::Add(_, rhs) => {
                                                let rhs1 = mem::take(rhs.as_mut());
                                                *rhs = Box::new(Value::Mul(
                                                    Box::new(rhs1),
                                                    Box::new(Value::Copy(0)),
                                                ));
                                                rhs.simplify();
                                            }
                                            Value::Input { .. } | Value::Mul(..) => {
                                                unreachable!()
                                            }
                                        }
                                    }
                                    let bb = body.drain(..).next().unwrap();
                                    *self = bb;
                                    return;
                                }
                            }
                            *condition = Condition::Count(Value::Copy(0));
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
            Ast::Inc => self.cell_mut().add(1),
            Ast::Dec => self.cell_mut().add(255),
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
                Effect::Output(value) => value.rebase(self),
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
            cell.rebase(self);
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
        let index = self.origin_index as isize + offset;
        if index < 0 {
            let n = index.unsigned_abs();
            self.memory.reserve(n);
            let offset = offset - self.origin_index as isize;
            for i in 0..n {
                self.memory.push_front(Value::Copy(offset - i as isize));
            }
            self.origin_index += n;
        } else if index as usize >= self.memory.len() {
            let n = index as usize - self.memory.len() + 1;
            self.memory.reserve(n);
            let offset = self.memory.len() - self.origin_index;
            for i in 0..n {
                self.memory.push_back(Value::Copy((offset + i) as isize));
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

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::{
        ir::{BasicBlock, Condition, Effect, Ir},
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
        let expected = BasicBlock {
            memory: VecDeque::from([
                Value::Add(Box::new(Value::Copy(-1)), Box::new(Value::Const(255))),
                Value::Input { id: 0 },
            ]),
            effects: vec![
                Effect::GuardShift(1),
                Effect::GuardShift(-1),
                Effect::Output(Value::Add(
                    Box::new(Value::Copy(0)),
                    Box::new(Value::Const(2)),
                )),
                Effect::Input { id: 0 },
                Effect::GuardShift(2),
            ],
            origin_index: 1,
            offset: 2,
            guarded_left: -1,
            guarded_right: 2,
            inputs: 1,
        };
        assert_eq!(bb, expected);
    }

    #[test]
    fn lower() {
        // Excerpt from https://www.brainfuck.org/collatz.b
        let src = b"[-[<->-]+[<<<<]]<[>+<-]";
        let ast = Ast::parse(src).unwrap();
        let ir = Ir::lower(&ast);
        let expected = vec![
            Ir::Loop {
                condition: Condition::WhileNonZero,
                body: vec![
                    Ir::BasicBlock(BasicBlock {
                        memory: VecDeque::from([Value::Add(
                            Box::new(Value::Copy(0)),
                            Box::new(Value::Const(255)),
                        )]),
                        effects: vec![],
                        origin_index: 0,
                        offset: 0,
                        guarded_left: 0,
                        guarded_right: 0,
                        inputs: 0,
                    }),
                    Ir::Loop {
                        condition: Condition::WhileNonZero,
                        body: vec![Ir::BasicBlock(BasicBlock {
                            memory: VecDeque::from([
                                Value::Add(Box::new(Value::Copy(-1)), Box::new(Value::Const(255))),
                                Value::Add(Box::new(Value::Copy(0)), Box::new(Value::Const(255))),
                            ]),
                            effects: vec![Effect::GuardShift(-1)],
                            origin_index: 1,
                            offset: 0,
                            guarded_left: -1,
                            guarded_right: 0,
                            inputs: 0,
                        })],
                    },
                    Ir::BasicBlock(BasicBlock {
                        memory: VecDeque::from([Value::Add(
                            Box::new(Value::Copy(0)),
                            Box::new(Value::Const(1)),
                        )]),
                        effects: vec![],
                        origin_index: 0,
                        offset: 0,
                        guarded_left: 0,
                        guarded_right: 0,
                        inputs: 0,
                    }),
                    Ir::Loop {
                        condition: Condition::WhileNonZero,
                        body: vec![Ir::BasicBlock(BasicBlock {
                            memory: VecDeque::from([]),
                            effects: vec![
                                Effect::GuardShift(-1),
                                Effect::GuardShift(-2),
                                Effect::GuardShift(-3),
                                Effect::GuardShift(-4),
                            ],
                            origin_index: 0,
                            offset: -4,
                            guarded_left: -4,
                            guarded_right: 0,
                            inputs: 0,
                        })],
                    },
                ],
            },
            Ir::BasicBlock(BasicBlock {
                memory: VecDeque::from([]),
                effects: vec![Effect::GuardShift(-1)],
                origin_index: 0,
                offset: -1,
                guarded_left: -1,
                guarded_right: 0,
                inputs: 0,
            }),
            Ir::Loop {
                condition: Condition::WhileNonZero,
                body: vec![Ir::BasicBlock(BasicBlock {
                    memory: VecDeque::from([
                        Value::Add(Box::new(Value::Copy(0)), Box::new(Value::Const(255))),
                        Value::Add(Box::new(Value::Copy(1)), Box::new(Value::Const(1))),
                    ]),
                    effects: vec![Effect::GuardShift(1)],
                    origin_index: 0,
                    offset: 0,
                    guarded_left: 0,
                    guarded_right: 1,
                    inputs: 0,
                })],
            },
        ];
        assert_eq!(ir, expected);
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
        let expected = vec![Ir::BasicBlock(BasicBlock {
            memory: VecDeque::from([Value::Const(0)]),
            effects: vec![],
            origin_index: 0,
            offset: 0,
            guarded_left: 0,
            guarded_right: 0,
            inputs: 0,
        })];
        assert_eq!(ir, expected);

        let src = b"[->+<]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expected = vec![Ir::BasicBlock(BasicBlock {
            memory: VecDeque::from([
                Value::Const(0),
                Value::Add(Box::new(Value::Copy(1)), Box::new(Value::Copy(0))),
            ]),
            effects: vec![Effect::GuardShift(1)],
            origin_index: 0,
            offset: 0,
            guarded_left: 0,
            guarded_right: 1,
            inputs: 0,
        })];
        assert_eq!(ir, expected);

        let src = b"[->+++<]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expected = vec![Ir::BasicBlock(BasicBlock {
            memory: VecDeque::from([
                Value::Const(0),
                Value::Add(
                    Box::new(Value::Copy(1)),
                    Box::new(Value::Mul(
                        Box::new(Value::Copy(0)),
                        Box::new(Value::Const(3)),
                    )),
                ),
            ]),
            effects: vec![Effect::GuardShift(1)],
            origin_index: 0,
            offset: 0,
            guarded_left: 0,
            guarded_right: 1,
            inputs: 0,
        })];
        assert_eq!(ir, expected);

        let src = b"[.-]";
        let mut ir = Ir::lower(&Ast::parse(src).unwrap());
        Ir::optimize_root(&mut ir);
        let expected = vec![Ir::Loop {
            condition: Condition::Count(Value::Copy(0)),
            body: vec![Ir::BasicBlock(BasicBlock {
                memory: VecDeque::from([Value::Add(
                    Box::new(Value::Copy(0)),
                    Box::new(Value::Const(255)),
                )]),
                effects: vec![Effect::Output(Value::Copy(0))],
                origin_index: 0,
                offset: 0,
                guarded_left: 0,
                guarded_right: 0,
                inputs: 0,
            })],
        }];
        assert_eq!(ir, expected);
    }
}
