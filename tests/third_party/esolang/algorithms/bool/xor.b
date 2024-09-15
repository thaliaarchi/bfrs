[z = x xor y (boolean, logical)

Attribution: Yuval Meshorer <https://esolangs.org/wiki/User:YuvalM>

Consumes x and y. Makes z 1 (true) or 0 (false) if x does not equal y. Finishes
at y.

Layout: x y z]

z>>[-]
x<<[y>-
 x<-]
y>[z>+
 y<[-]]
