predicate ezcp_bin_packing(array[int] of var int: l,
                                  array[int] of var int: bin,
                                  array[int] of int: w);

predicate fzn_bin_packing_load(array[int] of var int: l,
                               array[int] of var int: bin,
                               array[int] of int: w) =
    ezcp_bin_packing(l,bin,w);

