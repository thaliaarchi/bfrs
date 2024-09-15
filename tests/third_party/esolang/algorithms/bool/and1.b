[z = x and y (boolean, logical) (wrapping)

Attribution: Sunjay Varma <https://esolangs.org/wiki/User:Sunjay>

Consumes x and y (leaves them as zero at the end of the algorithm) and stores
the result in z. For short-circuit evaluation, don't evaluate x or y until just
before they are used.

The algorithm returns either 0 (false) or 1 (true).

Layout: x y z]

z>>[-]
x<<[
 y>[z>+y<-]
 x<-
]
y>[-]
