# Recursive loop peeling

Suppose we have a Brainfuck program, which shifts cells right a number of cells:

```brainfuck
[
 >>>
 [-]
 <[->+<]
 <[->+<]
 <-
]
```

It corresponds to the following high-level code, with `w`, `x`, `y`, and `z`
assigned to increasing cells:

```rust
while w != 0 {
  z = y
  y = x
  x = 0
  w -= 1
}
```

It illustrates recursive loop peeling well and is structurally similar to a more
common multiply program:

```brainfuck
[
 >[>+>+<<-]
 >[<+>-]
 <<-
]
```

```rust
while w != 0 {
  z += x
  x += y
  y = 0
  w -= 1
}
```

First, note that after the first iteration, `x` becomes loop-invariant; after
the second, `y` becomes loop-invariant; and after the third, `z` becomes
loop-invariant.

```rust
while w != 0 {
  z = y // Invariant after iterating thrice
  y = x // Invariant after iterating twice
  x = 0 // Invariant after iterating once
  w -= 1
}
```

If we peel the first three iterations of the loop, we can convert the loop tail
to its closed form. Generalized, this recursion will always terminate, because
the stores that become loop-invariant are moved out, so the tail loop is
strictly decreasing in size. Let's do it.

Peel the first iteration:

```rust
if w != 0 {
  z = y
  y = x
  x = 0
  w -= 1
  while w != 0 {
    z = y
    y = x
    w -= 1
  }
}
```

Peel the second iteration:

```rust
if w != 0 {
  z = y
  y = x
  x = 0
  w -= 1
  if w != 0 {
    z = y
    y = x
    w -= 1
    while w != 0 {
      z = y
      w -= 1
    }
  }
}
```

Peel the third iteration:

```rust
if w != 0 {
  z = y
  y = x
  x = 0
  w -= 1
  if w != 0 {
    z = y
    y = x
    w -= 1
    if w != 0 {
      z = y
      while w != 0 {
        w -= 1
      }
    }
  }
}
```

Convert the tail to its closed form:

```rust
if w != 0 {
  z = y
  y = x
  x = 0
  w -= 1
  if w != 0 {
    z = y
    y = x
    w -= 1
    if w != 0 {
      z = y
      w = 0
    }
  }
}
```

Unify `w -= 1` and `w = 0`.

```rust
if w != 0 {
  z = y
  y = x
  x = 0
  w -= 1
  if w != 0 {
    z = y
    y = x
    if w != 1 {
      z = y
    }
    w = 0
  }
}
```

Move `z = y` into else branch:

```rust
if w != 0 {
  z = y
  y = x
  x = 0
  w -= 1
  if w != 0 {
    if w != 1 {
      z = x
    } else {
      z = y
    }
    y = x
    w = 0
  }
}
```

Unify `w -= 1` with `w = 0`:

```rust
if w != 0 {
  z = y
  y = x
  x = 0
  if w != 1 {
    if w != 2 {
      z = x
    } else {
      z = y
    }
    y = x
  }
  w = 0
}
```

Move store to `x` later:

```rust
if w != 0 {
  z = y
  y = x
  if w != 1 {
    if w != 2 {
      z = 0
    } else {
      z = y
    }
    y = 0
  }
  x = 0
  w = 0
}
```

Move store to `y` later:

```rust
if w != 0 {
  z = y
  if w != 1 {
    if w != 2 {
      z = 0
    } else {
      z = x
    }
    y = 0
  } else {
    y = x
  }
  x = 0
  w = 0
}
```

Move store to `z` later:

```rust
if w != 0 {
  if w != 1 {
    if w != 2 {
      z = 0
    } else {
      z = x
    }
    y = 0
  } else {
    z = y
    y = x
  }
  x = 0
  w = 0
}
```

Recognize conditional move for `z`:

```rust
if w != 0 {
  if w != 1 {
    z = w != 2 ? 0 : x
    y = 0
  } else {
    z = y
    y = x
  }
  x = 0
  w = 0
}
```

Recognize conditional moves for `z` and `y`.

```rust
if w != 0 {
  z = w != 1 ? (w != 2 ? 0 : x) : y
  y = w != 1 ? 0 : x
  x = 0
  w = 0
}
```
