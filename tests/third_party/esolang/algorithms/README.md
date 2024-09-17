# Brainfuck algorithms on the Esolang wiki

The programs on the [Brainfuck algorithms](https://esolangs.org/wiki/Brainfuck_algorithms)
page. These programs are current as of revision [2024-09-17 00:18:39](https://esolangs.org/w/index.php?title=Brainfuck_algorithms&oldid=139541)
and authors are credited when attributed in the text.

In programs that use placeholders for shifts or constants, I have inserted
values and placed the original templates in a corresponding .b.orig file.
Programs for which I could not deduce the meaning of their placeholder notation
do not have a corresponding .b file.

- [Comment loop](https://esolangs.org/wiki/Brainfuck_algorithms#Comment_loop):
  comment/header_comment.b,
  comment/comment_after_clear.b,
  comment/comment_after_loop.b
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
- [x´ = not x (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_not_x_(boolean,_logical)):
  bool/not1_assign.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari),
  bool/not2_assign.b by [Sunjay Varma](https://esolangs.org/wiki/User:Sunjay),
  bool/not3_assign.b,
  bool/not4_assign.b by [Yuval Meshorer](https://esolangs.org/wiki/User:YuvalM),
  bool/not5_assign.b by User:A
- [y = not x (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#y_=_not_x_(boolean,_logical)):
  bool/not.b by [FSHelix](https://esolangs.org/wiki/User:FSHelix)
- [x´ = x and y (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_and_y_(boolean,_logical)):
  bool/and_assign.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari),
- [z = x and y (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#z_=_x_and_y_(boolean,_logical)_(wrapping)):
  bool/and1.b by [Sunjay Varma](https://esolangs.org/wiki/User:Sunjay),
  bool/and2.b by [Yuval Meshorer](https://esolangs.org/wiki/User:YuvalM)
- [z = x nand y (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#z_=_x_nand_y_(boolean,_logical)):
  bool/nand.b by [FSHelix](https://esolangs.org/wiki/User:FSHelix)
- [x´ = x or y (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_or_y_(boolean,_logical)):
  bool/or_assign1.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari),
  bool/or_assign2.b by [Yuval Meshorer](https://esolangs.org/wiki/User:YuvalM)
- [z = x or y (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#z_=_x_or_y_(boolean,_logical)):
  bool/or1.b by [Sunjay Varma](https://esolangs.org/wiki/User:Sunjay),
  bool/or2.b by [Yuval Meshorer](https://esolangs.org/wiki/User:YuvalM),
  bool/or{3,4}.b
- [x´ = x nor y (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#x%C2%B4_=_x_nor_y_(boolean,_logical)):
  bool/nor_assign.b by [FSHelix](https://esolangs.org/wiki/User:FSHelix)
- [z = x xor y (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#z_=_x_xor_y_(boolean,_logical)):
  bool/xor.b by [Yuval Meshorer](https://esolangs.org/wiki/User:YuvalM)
- [z = x xnor y (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#z_=_x_xnor_y_(boolean,_logical)):
  bool/xnor.b by [FSHelix](https://esolangs.org/wiki/User:FSHelix)
- [z = MUX(a, x, y) (boolean, logical)](https://esolangs.org/wiki/Brainfuck_algorithms#z_=_MUX(a,_x,_y)_(boolean,_logical)):
  bool/mux.b by [Yuval Meshorer](https://esolangs.org/wiki/User:YuvalM)
- [while (x) { code }](https://esolangs.org/wiki/Brainfuck_algorithms#while_(x)_{_code_}):
  control/while1.b.orig by [Sunjay Varma](https://esolangs.org/wiki/User:Sunjay),
  control/while2.b.orig by [Morgan Barrett](https://esolangs.org/wiki/User:Morganbarrett)
- [break and continue](https://esolangs.org/wiki/Brainfuck_algorithms#break_and_continue):
  control/break_and_continue.md by [Sunjay Varma](https://esolangs.org/wiki/User:Sunjay)
- [do { code } while (x)](https://esolangs.org/wiki/Brainfuck_algorithms#do_{_code_}_while_(x)):
  control/do_while.b.orig by [None1](https://esolangs.org/wiki/User:None1)
- [if (x) { code }](https://esolangs.org/wiki/Brainfuck_algorithms#if_(x)_{_code_}):
  control/if{1,2,3}.b
- [if (x) { code1 } else { code2 }](https://esolangs.org/wiki/Brainfuck_algorithms#if_(x)_{_code1_}_else_{_code2_}):
  control/if_else1.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari),
  control/if_else2.b by Daniel Marschall,
  control/if_else3.b by Ben-Arba
- [switch (x) {case 1: code1, case 2: code 2}](https://esolangs.org/wiki/Brainfuck_algorithms#switch_(x)_{case_1:_code1,_case_2:_code_2}):
  control/switch.b.orig
- [x = pseudo-random number](https://esolangs.org/wiki/Brainfuck_algorithms#x_=_pseudo-random_number):
  rand/lcg.b by [Jeffry Johnston](https://esolangs.org/wiki/User:Calamari),
  rand/simple.b by [Cinnamony](https://esolangs.org/wiki/User:Cinnamony)
- [Divmod](https://esolangs.org/wiki/Brainfuck_algorithms#Divmod):
  math/divmod{1,2}.b,
  math/divmod{3,4}.b by [FSHelix](https://esolangs.org/wiki/User:FSHelix),
  math/divmod5.b
- [Modulo](https://esolangs.org/wiki/Brainfuck_algorithms#Modulo):
  math/mod{1,2}.b
- [Print value of cell x as number (8-bit)](https://esolangs.org/wiki/Brainfuck_algorithms#Print_value_of_cell_x_as_number_(8-bit)):
  io/print_decimal.b by itchyny
- [Print value of cell x as number for ANY sized cell (eg 8bit, 100000bit etc)](https://esolangs.org/wiki/Brainfuck_algorithms#Print_value_of_cell_x_as_number_for_ANY_sized_cell_(eg_8bit,_100000bit_etc)):
  io/print_decimal_any_size{1,2}.b
- [Input a decimal number](https://esolangs.org/wiki/Brainfuck_algorithms#Input_a_decimal_number):
  io/read_decimal1.b by Urban Müller,
  io/read_decimal{2,3}.b by [Tommyaweosme](https://esolangs.org/wiki/User:Tommyaweosme),
- [Count up with step x, from y to infinity](https://esolangs.org/wiki/Brainfuck_algorithms#Count_up_with_step_x,_from_y_to_infinity):
  control/loop_stride.b
- [while(c=getchar()!=X)](https://esolangs.org/wiki/Brainfuck_algorithms#while(c=getchar()!=X)):
  io/read_until.b

License: [CC0 1.0 Universal Public Domain Dedication](https://esolangs.org/wiki/Esolang:Copyrights)
