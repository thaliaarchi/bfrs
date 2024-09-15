# Brainfuck algorithms on the Esolang wiki

The programs on the [Brainfuck algorithms](https://esolangs.org/wiki/Brainfuck_algorithms)
page. These programs are current as of revision [2024-09-15 04:33:59](https://esolangs.org/w/index.php?title=Brainfuck_algorithms&oldid=139329)
and authors are credited when attributed in the text.

In programs that use placeholders for shifts or constants, I have inserted
values and placed the original templates in a corresponding .b.orig file.
Programs for which I could not deduce the meaning of their placeholder notation
do not have a corresponding .b file.

- [Header comment](https://esolangs.org/wiki/Brainfuck_algorithms#Header_comment):
  comment/header_comment{1,2,3}.b
- [Read all characters into memory](https://esolangs.org/wiki/Brainfuck_algorithms#Read_all_characters_into_memory):
  io/read_all_chars.b
- [Read until newline/other char](https://esolangs.org/wiki/Brainfuck_algorithms#Read_until_newline/other_char):
  io/read_until_lf.b
- [Read until any of multiple chars](https://esolangs.org/wiki/Brainfuck_algorithms#Read_until_any_of_multiple_chars):
  io/read_until_chars.b
- [x = 0](https://esolangs.org/wiki/Brainfuck_algorithms#x_=_0):
  assign/clear.b,
  assign/clear_nowrap1.b by [quintopia](https://esolangs.org/wiki/User:Quintopia),
  assign/clear_nowrap2.b by [JungHwan Min](https://esolangs.org/wiki/User:JHM)
- [x = y](https://esolangs.org/wiki/Brainfuck_algorithms#x_=_y):
  assign/copy_assign.b
- [x´ = x + y](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_+_y):
  math/add_assign{1,2}.b
- [x´ = x - y](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_-_y):
  math/sub_assign.b
- [x´ = x * y](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_*_y):
  math/mul_assign.b
- [x´ = x * x](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_*_x):
  math/square_assign.b by Softengy
- [x´ = x / y](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_/_y):
  math/div_assign1.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari),
  math/div_assign2.b by Softengy
- [x´ = x<sup>y</sup>](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_xy):
  math/exp_assign.b by chad3814
- [swap x, y](https://esolangs.org/wiki/Brainfuck_algorithms#swap_x,_y):
  assign/swap{1,2}.b
- [x = -x](https://esolangs.org/wiki/Brainfuck_algorithms#x_=_-x):
  math/neg_assign.b, math/neg_assign_nowrap.b
- [x´ = not x (bitwise)](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_not_x_(bitwise)):
  bitwise/not_assign1.b, bitwise/not_assign2.b.orig, bitwise/not_assign_nowrap.b
- [Find a zeroed cell](https://esolangs.org/wiki/Brainfuck_algorithms#Find_a_zeroed_cell):
  shift/find_zero_{right,left}.b
- [Find a non-zeroed cell](https://esolangs.org/wiki/Brainfuck_algorithms#Find_a_non-zeroed_cell):
  shift/find_nonzero_{right,left}.b by [Epsilon](https://esolangs.org/wiki/User:Epsilon)
- [Move pointer x (empty) cells](https://esolangs.org/wiki/Brainfuck_algorithms#Move_pointer_x_(empty)_cells):
  shift/shift_dynamic_{right,left}.b by [Kman](https://esolangs.org/wiki/User:Kman)
- [x(y) = z (1-d array) (2 cells/array element)](https://esolangs.org/wiki/Brainfuck_algorithms#x(y)_=_z_(1-d_array)_(2_cells/array_element)):
  array/write_array_2-cell.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari)
- [x = y(z) (1-d array) (2 cells/array element)](https://esolangs.org/wiki/Brainfuck_algorithms#x_=_y(z)_(1-d_array)_(2_cells/array_element)):
  array/read_array_2-cell.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari)
- [x(y) = z (1-d array) (1 cell/array element)](https://esolangs.org/wiki/Brainfuck_algorithms#x(y)_=_z_(1-d_array)_(1_cell/array_element)):
  array/write_array_1-cell.b by [Tritonio](https://esolangs.org/wiki/User:Tritonio)
- [x = y(z) (1-d array) (1 cell/array element)](https://esolangs.org/wiki/Brainfuck_algorithms#x_=_y(z)_(1-d_array)_(1_cell/array_element)):
  array/read_array_1-cell.b by [Tritonio](https://esolangs.org/wiki/User:Tritonio)
- [x´ = x == y](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_==_y):
  compare/eq1_assign.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari),
  compare/eq2_assign.b
- [x´ = x != y](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_!=_y):
  compare/ne1_assign.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari),
  compare/ne2_assign.b by [Yuval Meshorer](https://esolangs.org/wiki/User:YuvalM)
- [x´ = x < y](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_%3C_y):
  compare/lt_assign.b by Ian Kelly
- [x´ = x <= y](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_%3C=_y):
  compare/le_assign.b by Ian Kelly
- [z = x > y](https://esolangs.org/wiki/Brainfuck_algorithms#z_=_x_%3E_y):
  compare/gt.b by [ais523](https://esolangs.org/wiki/User:Ais523)
- [z = sign(x-y)](https://esolangs.org/wiki/Brainfuck_algorithms#z_=_sign(x-y)):
  compare/cmp_nowrap.b by [quintopia](https://esolangs.org/wiki/User:Quintopia)

License: [CC0 1.0 Universal Public Domain Dedication](https://esolangs.org/wiki/Esolang:Copyrights)
