[x´ = not x (boolean, logical)

Attribution: Sunjay Varma <https://esolangs.org/wiki/User:Sunjay>

Another version for when you can consume x (mutate its value). Also assumes that
x is either 0 or 1. If you do not want to consume x, you can still use this
algorithm. Just copy x to another cell, then apply the operation. The algorithm
returns either 0 (false) or 1 (true).

Layout: x temp0]

temp0>[-]+
x<[-temp0>-x<]temp0>[x<+temp0>-]
