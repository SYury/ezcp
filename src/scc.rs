fn calc_order(v: usize, gr: &Vec<Vec<usize>>, used: &mut Vec<bool>, order: &mut Vec<usize>) {
    used[v] = true;
    for u in gr[v].iter().cloned() {
        if !used[u] {
            calc_order(u, gr, used, order);
        }
    }
    order.push(v);
}
fn mark_component(
    v: usize,
    gr: &Vec<Vec<usize>>,
    used: &mut Vec<bool>,
    component: &mut Vec<usize>,
) {
    used[v] = true;
    component.push(v);
    for u in gr[v].iter().cloned() {
        if !used[u] {
            mark_component(u, gr, used, component);
        }
    }
}
pub fn compute_scc(gr: &Vec<Vec<usize>>) -> Vec<Vec<usize>> {
    let n = gr.len();
    let mut grt = vec![Vec::new(); n];
    let mut order = Vec::with_capacity(n);
    let mut used = vec![false; n];
    for v in 0..n {
        for u in gr[v].iter().cloned() {
            grt[u].push(v);
        }
    }
    for v in 0..n {
        if !used[v] {
            calc_order(v, &gr, &mut used, &mut order);
        }
    }
    order.reverse();
    used.fill(false);
    let mut ans = Vec::new();
    for v in order.drain(..) {
        let mut component = Vec::new();
        mark_component(v, &grt, &mut used, &mut component);
        ans.push(component);
    }
    ans
}
