use crate::solver::Solver;

pub trait Constraint {
    fn satisfied(&self) -> bool;
    /// this function is run whenever the constraint is added to solver
    fn create_propagators(&self, solver: &mut Solver);
}
