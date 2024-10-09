# bfrs2

An optimizing Brainfuck compiler. Its IR is a Control Data Flow Graph with
owned and structured CFG nodes which are mutated in place, and floating data
nodes for pure operations which are managed immutably in an e-graph. It
recognizes add-assign loops and converts them to closed form multiplies, peels
quasi-invariant[^peel-paper] statements recursively, and propagates constants
to copies using them.

As an example of its optimization capabilities, this [multiply](../tests/mul.b)
snippet:

```brainfuck
[
 >[>+>+<<-]
 >[<+>-]
 <<-
]
```

corresponds to this C-like pretty-printed IR:

```rust
while p[0] != 0 {
    p += 1
    while p[0] != 0 {
        let c0 = p[0]
        let c1 = p[1]
        let c2 = p[2]
        p[0] = c0 - 1
        p[1] = c1 + 1
        p[2] = c2 + 1
    }
    p += 1
    while p[0] != 0 {
        let cn1 = p[-1]
        let c0 = p[0]
        p[-1] = cn1 + 1
        p[0] = c0 - 1
    }
    let cn2 = p[-2]
    p[-2] = cn2 - 1
    p -= 2
}
```

bfrs2 transforms it to the following, by converting the inner loops to copies,
peeling the outer loop once, then converting the outer loop to a multiply:

```rust
if p[0] != 0 {
    let c0 = p[0]
    let c1 = p[1]
    let c2 = p[2]
    let c3 = p[3]
    p[0] = 0
    p[1] = c2 + c1
    p[2] = 0
    p[3] = c3 + c1 + (c2 + c1) * (c0 - 1)
}
```

Similarly, my [move-right](../tests/move_right.b) example:

```brainfuck
[
 >>>
 [-]
 <[->+<]
 <[->+<]
 <-
]
```

corresponds to this C-like pretty-printed IR:

```rust
while p[0] != 0 {
    p += 3
    while p[0] != 0 {
        let c0 = p[0]
        p[0] = c0 - 1
    }
    p -= 1
    while p[0] != 0 {
        let c0 = p[0]
        let c1 = p[1]
        p[0] = c0 - 1
        p[1] = c1 + 1
    }
    p -= 1
    while p[0] != 0 {
        let c0 = p[0]
        let c1 = p[1]
        p[0] = c0 - 1
        p[1] = c1 + 1
    }
    let cn1 = p[-1]
    p[-1] = cn1 - 1
    p -= 1
}
```

bfrs2 transforms it to the following, by converting inner loops to copies,
eliminating shifts, then peeling the outer loop three times, until it can be
converted to its closed form:

```rust
if p[0] != 0 {
    let c0 = p[0]
    let c1 = p[1]
    let c2 = p[2]
    p[0] = c0 - 1
    p[1] = 0
    p[2] = c1
    p[3] = c2
    if p[0] != 0 {
        let c0 = p[0]
        let c2 = p[2]
        p[0] = c0 - 1
        p[2] = 0
        p[3] = c2
        if p[0] != 0 {
            p[0] = 0
            p[3] = 0
        }
    }
}
```

Those are exactly what's dumped, only with shift guards removed.

To get this, I ended up rewriting everything from the ground up, since bfrs1 had
accrued bits of cruft from changing designs. I had plans to make it a graph IR,
mutably rewriting nodes, but that wasn't comfortable with a Rusty arena
approach, so I ditched that and committed to a tree design, to see how far it
could get.

I've got three passes and, unfortunately, their intersection interacts
unsoundly. I have to toggle passes to get the above output, so there's bugs to
fix. Ultimately, passes are difficult to construct and debug, and I am moving
to an [e-graph IR](../docs/e-graph.md) without equality saturation, so that
rewrites preserve the old versions.

[^peel-paper]: Jean-Yves Moyen, Thomas Rubiano, and Thomas Seiller.
  [“Loop Quasi-Invariant Chunk Motion by peeling with statement composition”](https://www.cs.bham.ac.uk/~zeilbern/lola2017/abstracts/LOLA_2017_paper_1.pdf).
  2017
