[y = not x (boolean, logical)

Attribution: FSHelix <https://esolangs.org/wiki/User:FSHelix>

A harder-to-understand version that actually is "y = not x", which preserves x
but needs 3 continuous cells in total. Maybe using it for calculating "y = not
x" is not necessary, but I think this idea will be quite useful in some cases.
In fact the idea is also embodied in other codes in this page.

#Define these 3 cells as x, y=1 and t=0.]

x>y[-]+>t[-]<<x
[>y[-]]>[>t]

[According to whether x==0 or not, there are two different run modes because the
position of the pointer changes in the "[]" loop.

The following is the process of the second line, "*" means the pointer is here.
If x==0:                           If x!=0:
                  x  y  t                            x  y  t
                 *0  1  0                           *1  1  0
[>y[-]]          *0  1  0          [>y[-]]           1 *0  0
[>y[-]]>          0 *1  0          [>y[-]]>          1  0 *0
[>y[-]]>[>t]      0  1 *0          [>y[-]]>[>t]      1  0 *0
]
