[x(y) = z (1-d array) (1 cell/array element)

Attribution: Tritonio

The cells representing space, index1, index2 and Data must be contiguous and
initially empty (zeroed), with space being the leftmost cell and Data the
rightmost, followed by adequate memory for the array. Each array element
requires 1 memory cell. The pointer ends at space. index1, index2 and Data are
zeroed at the end.

Layout: y z space index1 index2 Data ...]

>z[->space+>>>data+z<<<<]space>[-z<+space>]
y<<[-space>>+index1>+y<<<]space>>[-y<<+space>>]
y<<[-space>>+index2>>+y<<<<]space>>[-y<<+space>>]
>[>>>[-<<<<+>>>>]<[->+<]<[->+<]<[->+<]>-]
>>>[-]<[->+<]<
[[-<+>]<<<[->>>>+<<<<]>>-]<<

[For an explanation on how this algorithm works read this article.
<https://www.inshame.com/2008/02/efficient-brainfuck-tables.html>]
