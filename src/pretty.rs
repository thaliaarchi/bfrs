use std::fmt::{self, Debug, Formatter, Write};

use crate::{
    graph::{hash_arena::ArenaRef, Graph},
    ir::{Cfg, Condition, Ir},
    node::Node,
    region::{Effect, Region},
};

struct PrettyPrinter<'a> {
    w: &'a mut (dyn Write + 'a),
    indent_buf: String,
}

impl<'a> PrettyPrinter<'a> {
    const INDENT: &'static str = "    ";

    fn new(w: &'a mut (dyn Write + 'a)) -> Self {
        PrettyPrinter {
            w,
            indent_buf: Self::INDENT.repeat(4),
        }
    }

    fn pretty_blocks(&mut self, blocks: &[Cfg], g: &Graph, indent: usize) -> fmt::Result {
        if let [Cfg::BasicBlock(bb)] = blocks {
            self.pretty_region(bb, g, indent)
        } else if let [block] = blocks {
            self.pretty_ir(block, g, indent)
        } else {
            for block in blocks {
                self.pretty_ir(block, g, indent)?;
            }
            Ok(())
        }
    }

    fn pretty_ir(&mut self, ir: &Cfg, g: &Graph, indent: usize) -> fmt::Result {
        match ir {
            Cfg::BasicBlock(bb) => {
                self.indent(indent)?;
                write!(self.w, "{{\n")?;
                self.pretty_region(bb, g, indent + 1)?;
                self.indent(indent)?;
                write!(self.w, "}}\n")
            }
            Cfg::Loop { condition, body } => {
                self.indent(indent)?;
                match *condition {
                    Condition::WhileNonZero => write!(self.w, "while @0 != 0")?,
                    Condition::IfNonZero => write!(self.w, "if @0 != 0")?,
                    Condition::Count(count) => write!(self.w, "repeat {:?} times", g.get(count))?,
                }
                write!(self.w, " {{\n")?;
                self.pretty_blocks(body, g, indent + 1)?;
                self.indent(indent)?;
                write!(self.w, "}}\n")
            }
        }
    }

    fn pretty_region(&mut self, region: &Region, g: &Graph, indent: usize) -> fmt::Result {
        // Write effects and track discrepancies between them and the statistic
        // fields.
        let mut guarded_left = 0;
        let mut guarded_right = 0;
        let mut inputs = 0;
        for effect in &region.effects {
            self.indent(indent)?;
            match *effect {
                Effect::Output(value) => {
                    write!(self.w, "output {:?}", g.get(value))?;
                }
                Effect::Input(value) => {
                    let value = g.get(value);
                    write!(self.w, "{:?} = input", value)?;
                    if let Node::Input { id } = *value {
                        if id != inputs {
                            write!(self.w, " # BUG: unordered")?;
                        }
                        inputs = id + 1;
                    } else {
                        write!(self.w, " # BUG: invalid type")?;
                    };
                }
                Effect::GuardShift(offset) => {
                    write!(self.w, "guard_shift {offset}")?;
                    if offset < 0 && offset < guarded_left {
                        guarded_left = offset;
                    } else if offset > 0 && offset > guarded_right {
                        guarded_right = offset;
                    } else if offset == 0 {
                        write!(self.w, " # BUG: zero")?;
                    } else {
                        write!(self.w, " # BUG: unordered")?;
                    }
                }
            }
            write!(self.w, "\n")?;
        }
        let memory = &region.memory;
        if memory.guarded_left() != guarded_left
            || memory.guarded_right() != guarded_right
            || region.inputs != inputs
        {
            self.indent(indent)?;
            write!(self.w, "# BUG:")?;
            if memory.guarded_left() != guarded_left {
                write!(self.w, " guarded_left={}", memory.guarded_left())?;
            }
            if memory.guarded_right() != guarded_right {
                write!(self.w, " guarded_right={}", memory.guarded_right())?;
            }
            if region.inputs != inputs {
                write!(self.w, " inputs={}", region.inputs)?;
            }
            write!(self.w, "\n")?;
        }

        for (offset, node) in memory.iter() {
            let node = g.get(node);
            if *node != Node::Copy(offset) {
                self.indent(indent)?;
                write!(self.w, "@{offset} = {:?}\n", node)?;
            }
        }

        if memory.offset() != 0 {
            self.indent(indent)?;
            write!(self.w, "shift {}\n", memory.offset())?;
        }
        Ok(())
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

impl Debug for ArenaRef<'_, Node> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        fn group(f: &mut Formatter<'_>, grouped: bool, node: ArenaRef<'_, Node>) -> fmt::Result {
            if grouped {
                write!(f, "({node:?})")
            } else {
                write!(f, "{node:?}")
            }
        }
        let g = self.graph();
        match *self.value() {
            Node::Copy(offset) => write!(f, "@{offset}"),
            Node::Const(value) => write!(f, "{}", value as i8),
            Node::Input { id } => write!(f, "in{id}"),
            Node::Add(lhs, rhs) => {
                let (lhs, rhs) = (g.get(lhs), g.get(rhs));
                write!(f, "{:?}", lhs)?;
                if let Node::Const(rhs) = *rhs {
                    if (rhs as i8) < 0 {
                        return write!(f, " - {}", (rhs as i8).unsigned_abs());
                    }
                }
                write!(f, " + ")?;
                group(f, matches!(*rhs, Node::Add(..)), rhs)
            }
            Node::Mul(lhs, rhs) => {
                let (lhs, rhs) = (g.get(lhs), g.get(rhs));
                group(f, matches!(*lhs, Node::Add(..)), lhs)?;
                write!(f, " * ")?;
                group(f, matches!(*rhs, Node::Add(..) | Node::Mul(..)), rhs)
            }
            Node::Array(ref elements) => {
                fn write_char(w: &mut dyn Write, ch: u8) -> fmt::Result {
                    match ch {
                        b'\0' => write!(w, "\\0"),
                        b'\t' => write!(w, "\\t"),
                        b'\n' => write!(w, "\\n"),
                        b'\r' => write!(w, "\\r"),
                        b'\\' => write!(w, "\\\\"),
                        _ if ch.is_ascii() && !ch.is_ascii_control() => write!(w, "{}", ch as char),
                        _ => write!(w, "\\x{ch:02x}"),
                    }
                }
                if elements
                    .iter()
                    .all(|&e| matches!(*g.get(e), Node::Const(_)))
                {
                    write!(f, "\"")?;
                    for &e in elements {
                        let Node::Const(ch) = *g.get(e) else {
                            unreachable!();
                        };
                        write_char(f, ch)?;
                    }
                    write!(f, "\"")
                } else {
                    write!(f, "[")?;
                    for (i, &e) in elements.iter().enumerate() {
                        if i != 0 {
                            write!(f, ", ")?;
                        }
                        let e = g.get(e);
                        if let Node::Const(ch) = *e {
                            write!(f, "'")?;
                            write_char(f, ch)?;
                            write!(f, "'")?;
                        } else {
                            write!(f, "{:?}", e)?;
                        }
                    }
                    write!(f, "]")
                }
            }
        }
    }
}

impl Ir {
    pub fn pretty(&self, g: &Graph) -> String {
        let mut s = String::new();
        self.write_pretty(&mut s, g).unwrap();
        s
    }

    pub fn write_pretty(&self, w: &mut dyn Write, g: &Graph) -> fmt::Result {
        PrettyPrinter::new(w).pretty_blocks(&self.blocks, g, 0)
    }

    pub fn compare_pretty(&self, expect: &str, g: &Graph) -> bool {
        let mut s = String::new();
        self.write_pretty(&mut s, g).unwrap();
        compare_pretty(&s, expect)
    }
}

impl Cfg {
    pub fn pretty(&self, g: &Graph) -> String {
        let mut s = String::new();
        self.write_pretty(&mut s, g).unwrap();
        s
    }

    pub fn write_pretty(&self, w: &mut dyn Write, g: &Graph) -> fmt::Result {
        PrettyPrinter::new(w).pretty_ir(self, g, 0)
    }

    pub fn compare_pretty(&self, expect: &str, g: &Graph) -> bool {
        compare_pretty(&self.pretty(g), expect)
    }
}

impl Region {
    pub fn pretty(&self, g: &Graph) -> String {
        let mut s = String::new();
        self.write_pretty(&mut s, g).unwrap();
        s
    }

    pub fn write_pretty(&self, w: &mut dyn Write, g: &Graph) -> fmt::Result {
        PrettyPrinter::new(w).pretty_region(self, g, 0)
    }

    pub fn compare_pretty(&self, expect: &str, g: &Graph) -> bool {
        compare_pretty(&self.pretty(g), expect)
    }
}

fn compare_pretty(got: &str, expect: &str) -> bool {
    let expect = unindent(expect);
    if got == expect {
        true
    } else {
        println!("<<<<<<< Expect");
        print!("{expect}");
        println!("=======");
        print!("{got}");
        println!(">>>>>>> Got");
        false
    }
}

fn unindent(s: &str) -> String {
    fn get_indent(s: &str) -> &str {
        let len = s
            .as_bytes()
            .iter()
            .position(|&b| b != b' ')
            .unwrap_or(s.len());
        &s[..(len / PrettyPrinter::INDENT.len()) * PrettyPrinter::INDENT.len()]
    }
    let s = s
        .trim_start_matches(|c| c == '\n')
        .trim_end_matches(|c| c == ' ' || c == '\n');
    let mut lines = s.lines();
    let first_indent = get_indent(lines.next().unwrap_or_default());
    let mut min_indent = lines
        .map(|line| get_indent(line))
        .min_by(|&a, &b| a.len().cmp(&b.len()))
        .unwrap_or(first_indent);
    if !first_indent.is_empty() && first_indent.len() < min_indent.len() {
        min_indent = first_indent;
    }
    let mut unindented = String::with_capacity(s.len());
    for line in s.lines() {
        unindented.push_str(line.strip_prefix(min_indent).unwrap_or(line));
        unindented.push_str("\n");
    }
    unindented
}
