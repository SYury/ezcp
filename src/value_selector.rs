use crate::domain::Domain;

pub trait ValueSelector {
    fn select(&self, dom: &dyn Domain) -> i64;
}

pub struct MinValueSelector {}

impl ValueSelector for MinValueSelector {
    fn select(&self, dom: &dyn Domain) -> i64 {
        dom.get_lb()
    }
}

pub struct MaxValueSelector {}

impl ValueSelector for MaxValueSelector {
    fn select(&self, dom: &dyn Domain) -> i64 {
        dom.get_ub()
    }
}
