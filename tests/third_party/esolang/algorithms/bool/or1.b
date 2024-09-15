[z = x or y (boolean, logical) (wrapping)

Attribution: Sunjay Varma <https://esolangs.org/wiki/User:Sunjay>

Consumes x and y (leaves them as zero at the end of the algorithm) and stores
the result in z. For short-circuit evaluation, don't evaluate x or y until just
before they are used.

If you don't care about short-circuit evaluation, temp0 can be removed
completely. If temp0 is removed and both x and y are 1, z will be 2, not 1. This
is usually not a problem since it is still non-zero, but you should keep that in
mind.

Or there's a way to fix it, add these codes to the end:

z[x+z[-]]
x[z+x-]

The algorithm returns either 0 (false) or 1 (true).

Layout: x y z temp0]

z>>[-]
temp0>[-]+
x<<<[
 z>>+
 temp0>-
 x<<<-
]
temp0>>>[-
 y<<[
  z>+
  y<-
 ]
]
y[-]