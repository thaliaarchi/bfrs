use std::{collections::VecDeque, mem};

use crate::Ast;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Ir {
    /// A basic block of non-branching instructions.
    BasicBlock(BasicBlock),
    /// A loop.
    Loop {
        /// The contained blocks.
        body: Vec<Ir>,
    },
}

/// Abstract model of a sub-slice of memory and the effects in a basic block.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicBlock {
    /// A sub-slice of the full memory.
    memory: VecDeque<AbstractCell>,
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

/// Abstract model of a cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AbstractCell {
    /// Copy the value of a cell from before this basic block.
    Copy { offset: isize },
    /// A constant value.
    Const { value: u8 },
    /// A value read from the user.
    Input { id: usize },
    /// Addition of two values.
    Add {
        lhs: Box<AbstractCell>,
        rhs: Box<AbstractCell>,
    },
}

/// An observable effect.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Effect {
    /// Printing a value.
    Output { value: AbstractCell },
    /// Reading from the user.
    Input { id: usize },
    /// Guarding that a shift can be performed.
    GuardShift { offset: isize },
}

impl Ir {
    pub fn lower(ast: &[Ast]) -> Vec<Self> {
        let mut ir = vec![];
        for inst in ast {
            if let Ast::Loop(body) = inst {
                ir.push(Ir::Loop {
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
                    self.effects.push(Effect::GuardShift {
                        offset: self.guarded_right,
                    });
                }
            }
            Ast::Left => {
                self.offset -= 1;
                if self.offset < self.guarded_left {
                    self.guarded_left = self.offset;
                    self.effects.push(Effect::GuardShift {
                        offset: self.guarded_left,
                    });
                }
            }
            Ast::Inc => self.cell_mut().add(1),
            Ast::Dec => self.cell_mut().add(255),
            Ast::Output => {
                let value = self.cell_mut().clone();
                self.effects.push(Effect::Output { value });
            }
            Ast::Input => {
                *self.cell_mut() = AbstractCell::Input { id: self.inputs };
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
                Effect::Output { value } => value.rebase(self),
                Effect::Input { id } => *id += self.inputs,
                Effect::GuardShift { offset } => {
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

    fn min_offset(&self) -> isize {
        -(self.origin_index as isize)
    }

    fn max_offset(&self) -> isize {
        self.min_offset() + self.memory.len() as isize
    }

    fn reserve(&mut self, offset: isize) {
        let index = self.origin_index as isize + offset;
        if index < 0 {
            let n = index.unsigned_abs();
            self.memory.reserve(n);
            let offset = offset - self.origin_index as isize;
            for i in 0..n {
                self.memory.push_front(AbstractCell::Copy {
                    offset: offset - i as isize,
                });
            }
            self.origin_index += n;
        } else if index as usize >= self.memory.len() {
            let n = index as usize - self.memory.len() + 1;
            self.memory.reserve(n);
            let offset = self.memory.len() - self.origin_index;
            for i in 0..n {
                self.memory.push_back(AbstractCell::Copy {
                    offset: (offset + i) as isize,
                });
            }
        }
    }

    fn cell_mut(&mut self) -> &mut AbstractCell {
        self.reserve(self.offset);
        &mut self.memory[(self.origin_index as isize + self.offset) as usize]
    }

    fn cell_copy(&self, offset: isize) -> AbstractCell {
        let index = self.origin_index as isize - offset;
        if 0 <= index && index < self.memory.len() as isize {
            self.memory[index as usize].clone()
        } else {
            AbstractCell::Copy { offset }
        }
    }
}

impl AbstractCell {
    fn add(&mut self, rhs: u8) {
        let lhs = match self {
            AbstractCell::Add { rhs, .. } => rhs.as_mut(),
            _ => self,
        };
        match lhs {
            AbstractCell::Const { value } => {
                *value = value.wrapping_add(rhs);
                return;
            }
            _ => {
                let lhs = mem::replace(self, AbstractCell::Copy { offset: isize::MAX });
                *self = AbstractCell::Add {
                    lhs: Box::new(lhs),
                    rhs: Box::new(AbstractCell::Const { value: rhs }),
                };
            }
        }
    }

    fn rebase(&mut self, bb: &BasicBlock) {
        match self {
            AbstractCell::Copy { offset } => {
                *self = bb.cell_copy(bb.offset + *offset);
            }
            AbstractCell::Const { .. } => {}
            AbstractCell::Input { id } => *id += bb.inputs,
            AbstractCell::Add { lhs, rhs } => {
                lhs.rebase(bb);
                rhs.rebase(bb);
                self.simplify();
            }
        }
    }

    fn simplify(&mut self) {
        match self {
            AbstractCell::Copy { .. } | AbstractCell::Const { .. } | AbstractCell::Input { .. } => {
            }
            AbstractCell::Add { lhs, rhs } => match (lhs.as_mut(), rhs.as_mut()) {
                (
                    AbstractCell::Const { value: lhs_value },
                    AbstractCell::Const { value: rhs_value },
                ) => {
                    *self = AbstractCell::Const {
                        value: *lhs_value + *rhs_value,
                    };
                }
                (AbstractCell::Const { .. }, _) => mem::swap(lhs, rhs),
                (
                    AbstractCell::Add {
                        lhs: lhs1,
                        rhs: lhs2,
                    },
                    _,
                ) => match lhs2.as_mut() {
                    AbstractCell::Const { value } => {
                        rhs.add(*value);
                        *lhs = lhs1.clone();
                    }
                    _ => {}
                },
                _ => {}
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use crate::{
        ir::{AbstractCell, BasicBlock, Effect, Ir},
        Ast,
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
                AbstractCell::Add {
                    lhs: Box::new(AbstractCell::Copy { offset: -1 }),
                    rhs: Box::new(AbstractCell::Const { value: 255 }),
                },
                AbstractCell::Input { id: 0 },
            ]),
            effects: vec![
                Effect::GuardShift { offset: 1 },
                Effect::GuardShift { offset: -1 },
                Effect::Output {
                    value: AbstractCell::Add {
                        lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                        rhs: Box::new(AbstractCell::Const { value: 2 }),
                    },
                },
                Effect::Input { id: 0 },
                Effect::GuardShift { offset: 2 },
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
                body: vec![
                    Ir::BasicBlock(BasicBlock {
                        memory: VecDeque::from([AbstractCell::Add {
                            lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                            rhs: Box::new(AbstractCell::Const { value: 255 }),
                        }]),
                        effects: vec![],
                        origin_index: 0,
                        offset: 0,
                        guarded_left: 0,
                        guarded_right: 0,
                        inputs: 0,
                    }),
                    Ir::Loop {
                        body: vec![Ir::BasicBlock(BasicBlock {
                            memory: VecDeque::from([
                                AbstractCell::Add {
                                    lhs: Box::new(AbstractCell::Copy { offset: -1 }),
                                    rhs: Box::new(AbstractCell::Const { value: 255 }),
                                },
                                AbstractCell::Add {
                                    lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                                    rhs: Box::new(AbstractCell::Const { value: 255 }),
                                },
                            ]),
                            effects: vec![Effect::GuardShift { offset: -1 }],
                            origin_index: 1,
                            offset: 0,
                            guarded_left: -1,
                            guarded_right: 0,
                            inputs: 0,
                        })],
                    },
                    Ir::BasicBlock(BasicBlock {
                        memory: VecDeque::from([AbstractCell::Add {
                            lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                            rhs: Box::new(AbstractCell::Const { value: 1 }),
                        }]),
                        effects: vec![],
                        origin_index: 0,
                        offset: 0,
                        guarded_left: 0,
                        guarded_right: 0,
                        inputs: 0,
                    }),
                    Ir::Loop {
                        body: vec![Ir::BasicBlock(BasicBlock {
                            memory: VecDeque::from([]),
                            effects: vec![
                                Effect::GuardShift { offset: -1 },
                                Effect::GuardShift { offset: -2 },
                                Effect::GuardShift { offset: -3 },
                                Effect::GuardShift { offset: -4 },
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
                effects: vec![Effect::GuardShift { offset: -1 }],
                origin_index: 0,
                offset: -1,
                guarded_left: -1,
                guarded_right: 0,
                inputs: 0,
            }),
            Ir::Loop {
                body: vec![Ir::BasicBlock(BasicBlock {
                    memory: VecDeque::from([
                        AbstractCell::Add {
                            lhs: Box::new(AbstractCell::Copy { offset: 0 }),
                            rhs: Box::new(AbstractCell::Const { value: 255 }),
                        },
                        AbstractCell::Add {
                            lhs: Box::new(AbstractCell::Copy { offset: 1 }),
                            rhs: Box::new(AbstractCell::Const { value: 1 }),
                        },
                    ]),
                    effects: vec![Effect::GuardShift { offset: 1 }],
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
}
