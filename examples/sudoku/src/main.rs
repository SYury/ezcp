use ezcp::alldifferent::AllDifferentConstraint;
use ezcp::constraint::Constraint;
use ezcp::solver::Solver;
use ezcp::value_selector::MinValueSelector;
use ezcp::variable_selector::FirstFailVariableSelector;
use std::boxed::Box;
use std::mem::swap;

const RNG_MOD: u64 = 1 << 31;
const RNG_MUL: u64 = 5;
const RNG_ADD: u64 = 11;

fn rand(x: u64) -> u64 {
    (x * RNG_MUL + RNG_ADD) % RNG_MOD
}

fn read_int() -> u64 {
    let mut input_line = String::new();
    std::io::stdin()
        .read_line(&mut input_line)
        .expect("No input!");
    input_line.trim().parse().expect("Input is not a valid unsigned 64-bit integer!")
}

fn generate_board(max_transforms: usize, mut seed: u64) -> [[u8; 9]; 9] {
    let mut board = [[0 as u8; 9]; 9];
    for block in 0..3 {
        let mut begin = block + 1;
        for row in 0..3 {
            for i in 0..9 {
                board[block * 3 + row][i] = ((begin + i - 1) % 9 + 1) as u8;
            }
            begin += 3;
        }
    }
    for _ in 0..max_transforms {
        seed = rand(seed);
        let action = (seed%19) as usize;
        if action < 9 {
            let stripe = action/3;
            let (mut i, mut j) = match action%3 {
                0 => (0, 1),
                1 => (0, 2),
                _ => (1, 2),
            };
            i += stripe * 3;
            j += stripe * 3;
            for k in 0..9 {
                let x = board[i][k];
                board[i][k] = board[j][k];
                board[j][k] = x;
            }
        } else if action < 18 {
            let stripe = (action - 9)/3;
            let (mut i, mut j) = match (action - 9)%3 {
                0 => (0, 1),
                1 => (0, 2),
                _ => (1, 2),
            };
            i += stripe * 3;
            j += stripe * 3;
            for k in 0..9 {
                board[k].swap(i, j);
            }
        } else {
            for i in 0..9 {
                for j in 0..i {
                    let x = board[i][j];
                    board[i][j] = board[j][i];
                    board[j][i] = x;
                }
            }
        }
    }
    board
}

fn generate_mask(n_masked: usize, mut seed: u64) -> [[bool; 9]; 9] {
    assert!(n_masked <= 81);
    let mut seq = Vec::<(usize, usize)>::with_capacity(81);
    for i in 0..9 {
        for j in 0..9 {
            seq.push((i, j));
        }
    }
    for i in 1..81 {
        seed = rand(seed);
        let j = (seed%((i + 1) as u64)) as usize;
        seq.swap(i, j);
    }
    let mut ans = [[false; 9]; 9];
    for i in 0..n_masked {
        let (x, y) = seq[i];
        ans[x][y] = true;
    }
    ans
}

fn main() {
    let seed = read_int();
    let board = generate_board(300, seed);
    let mask = generate_mask(50, seed);
    println!("Generated puzzle:");
    for i in 0..9 {
        let mut s = String::new();
        for j in 0..9 {
            if mask[i][j] {
                s.push('#');
            } else {
                s.push(char::from_digit(board[i][j] as u32, 10).unwrap());
            }
        }
        println!("{}", s);
    }
    let mut solver = Solver::new(Box::new(FirstFailVariableSelector{}), Box::new(MinValueSelector{}));
    let mut vars = Vec::with_capacity(81);
    for i in 0..9 {
        for j in 0..9 {
            if mask[i][j] {
                vars.push(solver.new_variable(1, 9, format!("cell({}, {})", i, j)));
            } else {
                vars.push(solver.new_variable(board[i][j] as i64, board[i][j] as i64, format!("cell({}, {})", i, j)));
            }
        }
    }
    for i in 0..9 {
        let mut v = Vec::with_capacity(9);
        for j in 0..9 {
            v.push(vars[i * 9 + j].clone());
        }
        solver.add_constraint(Box::new(AllDifferentConstraint::new(v)));
    }
    for j in 0..9 {
        let mut v = Vec::with_capacity(9);
        for i in 0..9 {
            v.push(vars[i * 9 + j].clone());
        }
        solver.add_constraint(Box::new(AllDifferentConstraint::new(v)));
    }
    for i in 0..3 {
        for j in 0..3 {
            let mut v = Vec::with_capacity(9);
            for di in 0..3 {
                for dj in 0..3 {
                    v.push(vars[(i * 3 + di) * 9 + j * 3 + dj].clone());
                }
            }
            solver.add_constraint(Box::new(AllDifferentConstraint::new(v)));
        }
    }
    assert!(solver.solve());
    println!("Solver found solution:");
    for i in 0..9 {
        let mut s = String::new();
        for j in 0..9 {
            s.push(char::from_digit(vars[i * 9 + j].borrow().value() as u32, 10).unwrap());
        }
        println!("{}", s);
    }
}
