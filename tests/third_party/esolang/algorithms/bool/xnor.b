[z = x xnor y (boolean, logical)

Attribution: FSHelix <https://esolangs.org/wiki/User:FSHelix>

Consumes x and y. Makes z 1 (true) or 0 (false) if x equal y. Finishes at y.]

z>>[-]+
x<<[
  y>-
  x<-
]
y>[
  z>-
  y<[-]
]
