[x´ = x nor y (boolean, logical)

Attribution: FSHelix <https://esolangs.org/wiki/User:FSHelix>

Consumes x and y and outputs 0 in x if both x and y are 1, else 1. Used an extra
cell "z" to avoid the overflow problem like the one mentioned in x = x or y.

Layout: x y z]

x[z>>+x<<[-]]
y>[z>+y<[-]]
z>[x<<+z>>[-]]
