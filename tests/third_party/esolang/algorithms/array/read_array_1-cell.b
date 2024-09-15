[x = y(z) (1-d array) (1 cell/array element)

Attribution: Tritonio

The cells representing space, index1, index2 and Data must be contiguous and
initially empty (zeroed), with space being the leftmost cell and Data the
rightmost, followed by adequate memory for the array. Each array element
requires 1 memory cell. The pointer ends at data. index1, index2 and Data are
zeroed at the end.

Layout: x z space index1 index2 Data ...]

z>[-space>+index1>+z<<]space>[-z<+space>]
z<[-space>+index2>>+z<<<]space>[-z<+space>]
>[>>>[-<<<<+>>>>]<<[->+<]<[->+<]>-]
>>>[-<+<<+>>>]<<<[->>>+<<<]>
[[-<+>]>[-<+>]<<<<[->>>>+<<<<]>>-]<<
x<<[-]
data>>>>>[-x<<<<<+data>>>>>]

[For an explanation on how this algorithm works read this article.
<https://www.inshame.com/2008/02/efficient-brainfuck-tables.html>]
