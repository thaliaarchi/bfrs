# bfrs

An optimizing compiler for Brainfuck.

Many patterns of loops are recognized and transformed into the closed form
arithmetic expressions they compute.

Architecturally, its intermediate representation is very similar to the
[design of Cranelift](https://vimeo.com/843540328): It has a control-flow graph
with effectful nodes strictly ordered in basic blocks and pure values ordered by
data dependencies, i.e., a control data flow graph. Pure values are managed in
an arena with global value-numbering and are rewritten to an idealized form on
construction.

See the [Brainfuck Program Corpus](https://github.com/thaliaarchi/bfcorpus) for
a large collection of Brainfuck programs.
