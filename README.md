# bfrs

An optimizing compiler for Brainfuck.

Many patterns of loops are recognized and transformed into the closed form
arithmetic expressions they compute.

Architecturally, its intermediate representation is very similar to the
[design of Cranelift](https://vimeo.com/843540328): It has a skeleton
control-flow graph with pure values floating in an arena with global
value-numbering. These pure nodes are rewritten to an idealized form on
construction.
