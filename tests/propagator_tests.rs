use ezcp::alldifferent::AllDifferentACPropagator;
use ezcp::propagator::Propagator;
use ezcp::search::SearchState;
use ezcp::variable::Variable;
use std::cell::RefCell;
use std::rc::Rc;

fn assert_domain(mut it: impl Iterator<Item = i64>, expected: Vec<i64>) {
    let mut it1 = expected.iter().cloned();
    loop {
        let x = it.next();
        let y = it1.next();
        if x.is_none() {
            if y.is_none() {
                break;
            } else {
                assert!(
                    false,
                    "Domain iterator ended, but expected value {}",
                    y.unwrap()
                );
            }
        }
        if y.is_none() {
            assert!(
                false,
                "Expected domain iterator to end, but got value {}",
                x.unwrap()
            );
        }
        let xval = x.unwrap();
        let yval = y.unwrap();
        assert_eq!(
            xval, yval,
            "Expected value {} in domain, but got {}",
            yval, xval
        );
    }
}

#[test]
fn test_alldifferent() {
    let fake_search_state = Rc::new(RefCell::new(SearchState::default()));
    let x = Rc::new(RefCell::new(Variable::new(
        fake_search_state.clone(),
        0,
        2,
        "x".to_string(),
    )));
    let y = Rc::new(RefCell::new(Variable::new(
        fake_search_state.clone(),
        0,
        0,
        "y".to_string(),
    )));
    let z = Rc::new(RefCell::new(Variable::new(
        fake_search_state,
        2,
        2,
        "z".to_string(),
    )));
    let mut p = AllDifferentACPropagator::new(vec![x.clone(), y.clone(), z.clone()], 0);
    p.propagate();
    assert_domain(x.borrow().iter(), vec![1]);
    assert_domain(y.borrow().iter(), vec![0]);
    assert_domain(z.borrow().iter(), vec![2]);
}
