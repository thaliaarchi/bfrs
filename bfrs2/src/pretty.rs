use std::{
    collections::BTreeSet,
    fmt::{self, Display, Formatter, Write},
};

use crate::{
    arena::{Arena, NodeId, NodeRef},
    block::{Block, Effect},
    cfg::Cfg,
    node::{BlockId, Node, Offset},
};

impl Cfg {
    pub fn pretty(&self, a: &Arena) -> String {
        let mut s = String::new();
        PrettyPrinter::new(&mut s, a).pretty_cfg(self, 0).unwrap();
        s
    }
}

impl Display for NodeRef<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        PrettyPrinter::new(f, self.arena()).pretty_node(self.id(), false)
    }
}

struct PrettyPrinter<'w, 'a> {
    w: &'w mut (dyn Write + 'w),
    indent_buf: String,
    a: &'a Arena,
}

impl<'w, 'a> PrettyPrinter<'w, 'a> {
    const INDENT: &'static str = "    ";

    fn new(w: &'w mut (dyn Write + 'w), a: &'a Arena) -> Self {
        PrettyPrinter {
            w,
            indent_buf: Self::INDENT.repeat(4),
            a,
        }
    }

    fn pretty_cfg(&mut self, cfg: &Cfg, indent: usize) -> fmt::Result {
        match cfg {
            Cfg::Block(block) => self.pretty_block(block, indent),
            Cfg::Seq(seq) => {
                for cfg in seq {
                    if let Cfg::Block(block) = cfg {
                        self.indent(indent)?;
                        write!(self.w, "{{\n")?;
                        self.pretty_block(block, indent + 1)?;
                        self.indent(indent)?;
                        write!(self.w, "}}\n")?;
                    } else {
                        self.pretty_cfg(cfg, indent)?;
                    }
                }
                Ok(())
            }
            Cfg::Loop(cfg) => {
                self.indent(indent)?;
                write!(self.w, "while p[0] != 0 {{\n")?;
                self.pretty_cfg(cfg, indent + 1)?;
                self.indent(indent)?;
                write!(self.w, "}}\n")
            }
        }
    }

    fn pretty_block(&mut self, block: &Block, indent: usize) -> fmt::Result {
        fn visit_copies(node: NodeRef<'_>, current_block: BlockId, copies: &mut BTreeSet<Offset>) {
            match *node.node() {
                Node::Copy(offset, block_id) => {
                    if block_id != current_block {
                        panic!("copy not from current block");
                    }
                    copies.insert(offset);
                }
                Node::Const(_) | Node::Input(_) => {}
                Node::Add(lhs, rhs) => {
                    visit_copies(node.get(lhs), current_block, copies);
                    visit_copies(node.get(rhs), current_block, copies);
                }
            }
        }

        for effect in &block.effects {
            self.indent(indent)?;
            self.pretty_effect(effect)?;
            write!(self.w, "\n")?;
        }
        let mut copies = BTreeSet::new();
        for (_, node) in block.iter_memory() {
            visit_copies(self.a.get(node), block.id, &mut copies);
        }
        for &copy in &copies {
            self.indent(indent)?;
            write!(self.w, "let ")?;
            self.pretty_copy(copy)?;
            write!(self.w, " = p[{}]\n", copy.0)?;
        }
        for (offset, node) in block.iter_memory() {
            if self.a[node] != Node::Copy(offset, block.id) {
                self.indent(indent)?;
                write!(self.w, "p[{}] = ", offset.0)?;
                self.pretty_node(node, true)?;
                writeln!(self.w)?;
            }
        }
        if block.offset != Offset(0) {
            self.indent(indent)?;
            write!(self.w, "shift({})\n", block.offset.0)?;
        }
        Ok(())
    }

    fn pretty_node(&mut self, node: NodeId, use_copies: bool) -> fmt::Result {
        let node = self.a.get(node);
        match *node.node() {
            Node::Copy(offset, _) => {
                if use_copies {
                    self.pretty_copy(offset)
                } else {
                    write!(self.w, "p[{}]", offset.0)
                }
            }
            Node::Const(c) => write!(self.w, "{}", c as i8),
            Node::Input(id) => write!(self.w, "in{}", id.0),
            Node::Add(lhs, rhs) => {
                self.pretty_node(lhs, use_copies)?;
                let rhs_node = &self.a[rhs];
                if let Node::Const(rhs) = *rhs_node {
                    if (rhs as i8) < 0 {
                        return write!(self.w, " - {}", (rhs as i8).unsigned_abs());
                    }
                }
                write!(self.w, " + ")?;
                self.group_node(rhs, matches!(rhs_node, Node::Add(..)), use_copies)
            }
        }
    }

    fn group_node(&mut self, node: NodeId, grouped: bool, use_copies: bool) -> fmt::Result {
        if grouped {
            write!(self.w, "(")?;
        }
        self.pretty_node(node, use_copies)?;
        if grouped {
            write!(self.w, ")")?;
        }
        Ok(())
    }

    fn pretty_copy(&mut self, copy: Offset) -> fmt::Result {
        if copy.0 < 0 {
            write!(self.w, "cn{}", copy.0.unsigned_abs())
        } else {
            write!(self.w, "c{}", copy.0)
        }
    }

    fn pretty_effect(&mut self, effect: &Effect) -> fmt::Result {
        match effect {
            Effect::Output(values) => {
                write!(self.w, "output(")?;
                self.pretty_array(values)?;
                write!(self.w, ")")
            }
            &Effect::Input(id) => write!(self.w, "let {} = input()", self.a.get(id)),
            &Effect::GuardShift(offset) => write!(self.w, "guard_shift({})", offset.0),
        }
    }

    fn pretty_array(&mut self, values: &[NodeId]) -> fmt::Result {
        if values.iter().all(|&v| matches!(self.a[v], Node::Const(_))) {
            write!(self.w, "\"")?;
            for &v in values {
                let Node::Const(b) = self.a[v] else {
                    unreachable!();
                };
                self.escape_char(b)?;
            }
            write!(self.w, "\"")
        } else {
            write!(self.w, "[")?;
            for (i, &v) in values.iter().enumerate() {
                if i != 0 {
                    write!(self.w, ", ")?;
                }
                let v = self.a.get(v);
                if let Node::Const(ch) = *v {
                    write!(self.w, "'")?;
                    self.escape_char(ch)?;
                    write!(self.w, "'")?;
                } else {
                    write!(self.w, "{v}")?;
                }
            }
            write!(self.w, "]")
        }
    }

    fn escape_char(&mut self, b: u8) -> fmt::Result {
        match b {
            b'\0' => write!(self.w, "\\0"),
            b'\t' => write!(self.w, "\\t"),
            b'\n' => write!(self.w, "\\n"),
            b'\r' => write!(self.w, "\\r"),
            b'\\' => write!(self.w, "\\\\"),
            _ if b.is_ascii() && !b.is_ascii_control() => {
                write!(self.w, "{}", b as char)
            }
            _ => write!(self.w, "\\x{b:02x}"),
        }
    }

    fn indent(&mut self, indent: usize) -> fmt::Result {
        let len = indent * Self::INDENT.len();
        self.indent_buf.reserve(len);
        while self.indent_buf.len() < len {
            self.indent_buf.push_str(Self::INDENT);
        }
        self.w.write_str(&self.indent_buf[..len])
    }
}
