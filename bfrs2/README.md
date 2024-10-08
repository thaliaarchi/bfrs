# bfrs2

An optimizing Brainfuck compiler. Its IR is a Control Data Flow Graph with
owned and structured CFG nodes which are mutated in place, and floating data
nodes for pure operations which are managed immutably in an arena. It recognizes
add-assign loops and converts them to closed form multiplies, peels
quasi-invariant[^peel-paper] statements recursively, and propagates constants
to copies using them.

Here's a [multiply](../tests/mul.b) snippet, optimized:

```brainfuck
[
 >[>+>+<<-]
 >[<+>-]
 <<-
]
```

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

And my [move-right](../tests/move_right.b) example, optimized:

```brainfuck
[
 >>>
 [-]
 <[->+<]
 <[->+<]
 <-
]
```

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
fix. Ultimately, passes are difficult to construct and debug, and I intend to
move to an [e-graph IR](../docs/e-graph.md) without equality saturation.

[^peel-paper]: Jean-Yves Moyen, Thomas Rubiano, and Thomas Seiller.
  [“Loop Quasi-Invariant Chunk Motion by peeling with statement composition”](https://www.cs.bham.ac.uk/~zeilbern/lola2017/abstracts/LOLA_2017_paper_1.pdf).
  2017
