include "bin_packing_capa.mzn";

predicate fzn_bin_packing(int: c,
                          array[int] of var int: bin,
                          array[int] of int: w) =
    let {
        set of int: idx = lb_array(bin)..ub_array(bin),
        array[idx] of int: capa = array1d(idx, [c|i in idx])
    } in
    bin_packing_capa(capa,bin,w);
