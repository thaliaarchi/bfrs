# Optimizing Wikipedia hello world

Goal: Prove that the loop has no net shift.

One strategy is to unroll the loop until it's all constant. In this case, that
works for all 8 iterations. Here's the first.

```ir
{
    guard_shift 1
    guard_shift 2
    guard_shift 3
    guard_shift 4
    guard_shift 5
    guard_shift 6
    @0 = 7
    @1 = 0
    @2 = 9
    @3 = 13
    @4 = 11
    @5 = 4
    @6 = 1
}
```

```ir
%start {
    @0 = 8
    mem0 = [8, 0...] offset=0
}
%loop: while @0 != 0 {
    {
        mem = phi [%start, mem0] [%loop, mem1]
        guard_shift 1
        guard_shift 2
        guard_shift 3
        guard_shift 4
        guard_shift 5
        guard_shift 6
        v1 = phi [%start, 0] [%loop, proj(mem1, 1)]
        v2 = phi [%start, 0] [%loop, proj(mem1, 2)]
        v3 = phi [%start, 0] [%loop, proj(mem1, 3)]
        v4 = phi [%start, 0] [%loop, proj(mem1, 4)]
        v5 = phi [%start, 0] [%loop, proj(mem1, 5)]
        v6 = phi [%start, 0] [%loop, proj(mem1, 6)]
        v1' = 0
        v2' = v2 + (v1 + 4) * 2 + 1
        v3' = v3 + (v1 + 4) * 3 + 1
        v4' = v4 + (v1 + 4) * 3 - 1
        v5' = v5 + v1 + 4
        v6' = v6 + 1
        shift 6
    }
    while @0 != 0 {
        guard_shift -1
        shift -1
    }
    {
        guard_shift -1
        @-1 = @-1 - 1
        shift -1
    }
}
```

Somehow solve for whether these are zero. If we peel the first loop, then we
know `mem[1] == 0` and `mem[2..=6] != 0`. Is it then true that `mem'[2..=6]`
will remain non-zero? Let's assume a consistent shift for now, so we can assume
`mem[1] == 0`. With that simplification, it's:

```ir
while phi [v0, v0'] != 0 {
    v0' = v0 - 1
    v1' = 0
    v2' = v2 + 9
    v3' = v3 + 13
    v4' = v4 + 11
    v5' = v5 + 4
    v6' = v6 + 1
}
```

Now, the goal is to find the first iteration when `mem'[2..=6]` will be 0. This
is a multiplication by the modular inverse! Then, the loop can be transformed
into that closed form prefix and the remainder!

```ir
n1 = 256 / gcd(9,  256) = 256
n2 = 256 / gcd(13, 256) = 256
n3 = 256 / gcd(11, 256) = 256
n4 = 256 / gcd(4,  256) = 64
n5 = 256 / gcd(1,  256) = 256
max_iterations = min(n1, n2, n3, n4, n5) = 64

v0 = 8
v1..=v6 = 0
loop max_iterations times {
    if v0 == 0 {
        break
    }
    v0' = v0 - 1
    v1' = 0
    v2' = v2 + 9
    v3' = v3 + 13
    v4' = v4 + 11
    v5' = v5 + 4
    v6' = v6 + 1
}
while … {
    # the original loop now
}
```

If the first iteration is peeled, and we derive it as the number of iterations
to get the rest, it comes out the same, when adding back 1 for the peeled
iteration. I'm not sure how to handle even in the general case here.

```ir
v0 = 7
v1 = 0
v2 = 9
v3 = 13
v4 = 11
v5 = 4
v6 = 1
max_iterations
    = min(
        9  * mod_inv(256 - 9)  mod 256 = 9  * 199 mod 256 = 255,
        13 * mod_inv(256 - 13) mod 256 = 13 * 59  mod 256 = 255,
        11 * mod_inv(256 - 11) mod 256 = 11 * 93  mod 256 = 255,
        4  * mod_inv(256 - 4)  mod 256 = 4  * ?   mod 256 = ?,
        1  * mod_inv(256 - 1)  mod 256 = 1  * 255 mod 256 = 255,
    )
```

Now, let's include the termination condition. It's less than the number of
iterations before there's an undesired zero, so it just gets the closed form.

```ir
n0 = 8 * mod_inv(1) = 8
n1 = 256 / gcd(9,  256) = 256
n2 = 256 / gcd(13, 256) = 256
n3 = 256 / gcd(11, 256) = 256
n4 = 256 / gcd(4,  256) = 64
n5 = 256 / gcd(1,  256) = 256
max_iterations = min(n0, n1, n2, n3, n4, n5) = 8

v0 = 8
v1..=v6 = 0
{
    v0' = v0 + 8 * -1
    v1' = 0
    v2' = v2 + 8 * 9
    v3' = v3 + 8 * 13
    v4' = v4 + 8 * 11
    v5' = v5 + 8 * 4
    v6' = v6 + 8 * 1
}
```

Simplified:

```ir
{
    v0 = 0
    v1 = 0
    v2 = 72
    v3 = 104
    v4 = 88
    v5 = 32
    v6 = 8
}
```

However, let's say there would be an undesired zero before v1 reaches 0. Let's
say v0 starts out at 199.

```ir
v0 = 199
v1..=v6 = 0
{
    v0' = v0 + 64 * -1
    v1' = 0
    v2' = v2 + 64 * 9
    v3' = v3 + 64 * 13
    v4' = v4 + 64 * 11
    v5' = v5 + 64 * 4
    v6' = v6 + 64 * 1
}
while v0 != 0 {
    # the original loop
}
```

Simplified:

```ir
{
    v0 = 135
    v1 = 0
    v2 = 64
    v3 = 64
    v4 = 192
    v5 = 0
    v6 = 64
}
while v0 != 0 {
    # the original loop
}
```

Now, let's say v0 is variable:

```ir
count = input

iters1 = min(count, 64)
v0 = count + iters1 * -1
v1 = 0
v2 = iters1 * 9
v3 = iters1 * 13
v4 = iters1 * 11
v5 = iters1 * 4
v6 = iters1 * 1

Iteration 64 is special, as it sets v5 = 0 and v4 becomes the counter. After
that, it loops as follows.
max_iterations2 = min(
    192 * mod_inv(1) = 192,
    (256 - 64) / gcd(9, 256 - 64) = 192,
    (256 - 0) / gcd(13, 256 - 0) = 256,
    (256 - 0) / gcd(11, 256 - 0) = 256,
    (256 - 0) / gcd(4,  256 - 0) = 64,
    (256 - 0) / gcd(1,  256 - 0) = 256)
loop {
    shift 4
    loop max_iterations2 times {
        v0' = v0 - 1
        v1' = 0
        v2' = v2 + 9
        v3' = v3 + 13
        v4' = v4 + 11
        v5' = v5 + 4
        v6' = v6 + 1
    }
}
```

It continually shifts right, so can be reduced to a panic.

```ir
…
if count >= 64 {
    panic_shift
}
```
