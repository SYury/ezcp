predicate ezcp_alldifferent(array[int] of var int: x);

predicate fzn_all_different_int(array[int] of var int: x) =
  ezcp_alldifferent(x);
