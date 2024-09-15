[Divmod

Attribution: FSHelix <https://esolangs.org/wiki/User:FSHelix>

Another version of divmod, does not preserve n. It uses 7 cells and more time to
calculate, and contains 2 layers of If-Else Structure(may be optimized in
future). However, it can deal with n, d = 0 ~ 255. Note that when d = 0, it
returns n/d = 0 and n%d = n. All inputs have been tested out.

# >n 1 d 1 0 0 0]

>+>>+<<<
[
 [
  ->->>>>>+<<<<-[>-]>[
   >>+>[-<<<<+>>>>]<<
  ]<[-]+<<
 ]>>>[>]<<<[-]+<
]

[# >0 1 d-n%d 1 0 n/d n%d

The pictures show its efficiency intuitively. It's obvious that versions above
are faster than this. The second version's maximum number of operations is 2.30
times this.]
