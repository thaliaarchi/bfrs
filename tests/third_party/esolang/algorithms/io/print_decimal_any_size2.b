[Print value of cell x as number for ANY sized cell (eg 8bit, 100000bit etc)

This alternative runs about a quarter as many BF instructions and is shorter.
However, a normally optimising interpreter runs it at about the same speed. It
requires about three times as many already cleaned cells two of which are to the
left of the cell to be printed. All cells, including the value printed, are
cleared after use.]

>> x
>+
[[-]<
  [->+<
    [->+<[->+<[->+<[->+<[->+<[->+<[->+<[->+<
      [->[-]>>+>+<<<]
    ]]]]]]]]<
  ]>>[>]++++++[-<++++++++>]>>
]<<<[.[-]<<<]
