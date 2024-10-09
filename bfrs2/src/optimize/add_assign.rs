use std::{
    mem,
    sync::atomic::{AtomicBool, Ordering},
};

use crate::{
    block::Block,
    cfg::Cfg,
    egraph::Graph,
    node::{Node, Offset},
};

static UNSOUND_OUTLINE_GUARDS: AtomicBool = AtomicBool::new(false);

impl Cfg {
    /// Converts loops, which have no net shift and add an odd constant to the
    /// current cell, to their closed form.
    pub fn opt_closed_form_add(&mut self, g: &mut Graph) {
        match self {
            Cfg::Block(_) => {}
            Cfg::Seq(seq) => {
                seq.iter_mut().for_each(|cfg| cfg.opt_closed_form_add(g));
                self.flatten(g);
            }
            Cfg::Loop(cfg) => {
                cfg.opt_closed_form_add(g);
                if let Cfg::Block(block) = cfg.as_mut() {
                    if let Some(factor) = block.closed_form_iter_factor(g) {
                        if let Some(has_guards) = block.is_pure() {
                            if block.opt_closed_form_add(factor, g) {
                                let Cfg::Loop(body) = mem::replace(self, Cfg::empty()) else {
                                    unreachable!();
                                };
                                if has_guards && !UNSOUND_OUTLINE_GUARDS.load(Ordering::Acquire) {
                                    *self = Cfg::If(body);
                                } else {
                                    *self = *body;
                                }
                            }
                        }
                    }
                }
            }
            Cfg::If(cfg_then) => {
                cfg_then.opt_closed_form_add(g);
            }
        }
    }
}

/// Set whether optimizations can outline shift guards, making them
/// unconditional. This is unsound in general.
pub fn unsound_outline_guards(enable: bool) {
    UNSOUND_OUTLINE_GUARDS.store(enable, Ordering::Release);
}

impl Block {
    /// Calculates the factor of the number of iterations this block would
    /// execute as the body of a loop. The number of iterations is the factor
    /// multiplied by the current cell. This can be calculated when the block
    /// has no net shift and an odd constant is added to the current cell.
    fn closed_form_iter_factor(&self, g: &Graph) -> Option<u8> {
        if self.offset == Offset(0) {
            if let Some(current) = self.get_cell(Offset(0)) {
                if let Node::Add(lhs, rhs) = g[current] {
                    if g[lhs] == Node::Copy(Offset(0), self.id) {
                        if let Node::Const(addend) = g[rhs] {
                            if let Some(factor) = mod_inverse(addend.wrapping_neg()) {
                                return Some(factor);
                            }
                        }
                    }
                }
            }
        }
        None
    }

    /// Converts a loop body, which has no net shift and adds an odd constant to
    /// the current cell, to its closed form. The block should be in a loop.
    fn opt_closed_form_add(&mut self, factor: u8, g: &mut Graph) -> bool {
        if !self
            .iter_memory()
            .all(|(offset, cell)| offset == Offset(0) || g.get(cell).is_add_assign(offset, self))
        {
            return false;
        }
        let block_id = self.id;
        let iters = Node::Mul(
            Node::Copy(Offset(0), block_id).insert_ideal(g),
            Node::Const(factor).insert_ideal(g),
        )
        .insert(g);
        self.iter_memory_mut(g, |offset, cell, a| {
            if offset == Offset(0) {
                Some(Node::Const(0).insert_ideal(a))
            } else {
                match a[cell] {
                    Node::Add(lhs, rhs) => {
                        debug_assert_eq!(a[lhs], Node::Copy(offset, block_id));
                        Some(Node::Add(lhs, Node::Mul(rhs, iters).insert(a)).insert(a))
                    }
                    _ => Some(cell),
                }
            }
        });
        true
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
