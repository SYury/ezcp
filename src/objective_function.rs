// function to minimize
pub trait ObjectiveFunction {
    fn eval(&self) -> i64;
    fn bound(&self) -> i64;
}
