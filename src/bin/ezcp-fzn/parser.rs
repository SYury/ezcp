use ezcp::alldifferent::AllDifferentConstraint;
use ezcp::binpacking::BinPackingConstraint;
use ezcp::config::Config;
use ezcp::linear::LinearInequalityConstraint;
use ezcp::logic::{AndConstraint, NegateConstraint, OrConstraint};
use ezcp::objective_function::SingleVariableObjective;
use ezcp::solver::Solver;
use ezcp::value_selector::MinValueSelector;
use ezcp::variable::Variable;
use ezcp::variable_selector::FirstFailVariableSelector;
use std::boxed::Box;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct MinizincParseResult {
    pub solver: Solver,
    pub output: Vec<String>,
}

fn int_array_or_ref(
    json: &serde_json::Value,
    arrays: &HashMap<String, Vec<i64>>,
) -> Result<Vec<i64>, String> {
    if let Some(s) = json.as_str() {
        if let Some(arr) = arrays.get(s) {
            Ok(arr.to_vec())
        } else {
            Err(format!("references array {}, but it doesn't exist.", s))
        }
    } else if let Some(arr) = json.as_array() {
        if arr.iter().any(|x| !x.is_i64()) {
            return Err("not a string or int array.".to_string());
        }
        Ok(arr.iter().map(|x| x.as_i64().unwrap()).collect::<Vec<_>>())
    } else {
        Err("not a string or int array.".to_string())
    }
}

fn var_array_or_ref(
    json: &serde_json::Value,
    arrays: &HashMap<String, Vec<String>>,
    vars: &HashMap<String, Rc<RefCell<Variable>>>,
) -> Result<Vec<Rc<RefCell<Variable>>>, String> {
    if let Some(s) = json.as_str() {
        if let Some(arr) = arrays.get(s) {
            if let Some(x) = arr.iter().find(|x| !vars.contains_key(*x)) {
                return Err(format!("references variable {}, but it doesn't exist", x));
            }
            Ok(arr
                .iter()
                .map(|x| vars.get(x).unwrap().clone())
                .collect::<Vec<_>>())
        } else {
            Err(format!("references array {}, but it doesn't exist.", s))
        }
    } else if let Some(arr) = json.as_array() {
        if arr.iter().any(|x| !x.is_string()) {
            return Err("not a string or string array.".to_string());
        }
        let names = arr
            .iter()
            .map(|x| x.as_str().unwrap())
            .collect::<Vec<&str>>();
        if let Some(x) = names.iter().find(|x| !vars.contains_key(**x)) {
            return Err(format!("references variable {}, but it doesn't exist", x));
        }
        Ok(names
            .iter()
            .map(|x| vars.get(*x).unwrap().clone())
            .collect::<Vec<_>>())
    } else {
        Err("not a string or string array.".to_string())
    }
}

fn var_array(
    arr: &[serde_json::Value],
    vars: &HashMap<String, Rc<RefCell<Variable>>>,
) -> Result<Vec<Rc<RefCell<Variable>>>, String> {
    if arr.iter().any(|x| !x.is_string()) {
        return Err("not a string array.".to_string());
    }
    let names = arr
        .iter()
        .map(|x| x.as_str().unwrap())
        .collect::<Vec<&str>>();
    if let Some(x) = names.iter().find(|x| !vars.contains_key(**x)) {
        return Err(format!("references variable {}, but it doesn't exist", x));
    }
    Ok(names
        .iter()
        .map(|x| vars.get(*x).unwrap().clone())
        .collect::<Vec<_>>())
}

pub fn parse(json: serde_json::Value) -> Result<MinizincParseResult, String> {
    let mut solver = Solver::new(Config::new(
        Box::new(MinValueSelector {}),
        Box::new(FirstFailVariableSelector {}),
    ));
    let mut vars = HashMap::<String, Rc<RefCell<Variable>>>::new();
    let mut arrays = HashMap::<String, Vec<i64>>::new();
    let mut string_arrays = HashMap::<String, Vec<String>>::new();
    let output: Vec<String>;

    if let Some(var_json0) = json.get("variables") {
        if let Some(var_json) = var_json0.as_object() {
            for (name, var) in var_json.iter() {
                if let Some(var_inner) = var.as_object() {
                    if !var_inner.contains_key("type") {
                        return Err(format!("variable {} has no type.", name));
                    }
                    if let Some(tp) = var_inner.get("type").unwrap().as_str() {
                        match tp {
                            "int" => {
                                if let Some(dom) =
                                    var_inner.get("domain").and_then(|d| d.as_array())
                                {
                                    if dom.len() != 1 {
                                        return Err(
                                            "Discontinious domains are not implemented, sorry."
                                                .to_string(),
                                        );
                                    }
                                    if let Some(range) = dom[0].as_array() {
                                        if range.len() != 2 {
                                            return Err(format!(
                                                "Invalid domain specification for variable {}",
                                                name
                                            ));
                                        }
                                        let l = range[0].as_i64().ok_or_else(|| {
                                            format!(
                                                "Invalid domain specification for variable {}",
                                                name
                                            )
                                        })?;
                                        let r = range[1].as_i64().ok_or_else(|| {
                                            format!(
                                                "Invalid domain specification for variable {}",
                                                name
                                            )
                                        })?;
                                        vars.insert(
                                            name.clone(),
                                            solver.new_variable(l, r, name.clone()),
                                        );
                                    } else {
                                        return Err(format!(
                                            "Invalid domain specification for variable {}",
                                            name
                                        ));
                                    }
                                } else {
                                    return Err(format!(
                                        "int variable {} has invalid domain.",
                                        name
                                    ));
                                }
                            }
                            "bool" => {
                                if var_inner.contains_key("domain") {
                                    return Err("Oops, it seems that bool vars in flatzinc may have domain... Parser must be fixed.".to_string());
                                } else {
                                    vars.insert(
                                        name.clone(),
                                        solver.new_variable(0, 1, name.clone()),
                                    );
                                }
                            }
                            _ => {
                                return Err(format!(
                                    "variable {} has unsupported type {}",
                                    name, tp
                                ));
                            }
                        }
                    } else {
                        return Err(format!("variable {} has non-string type record.", name));
                    }
                } else {
                    return Err(format!("info for variable {} is not a mapping.", name));
                }
            }
        } else {
            return Err("'variables' is not a mapping.".to_string());
        }
    } else {
        return Err("missing required field 'variables'.".to_string());
    }
    if let Some(arr_json) = json.get("arrays") {
        let arr_arr = arr_json
            .as_object()
            .ok_or_else(|| "'arrays' is not a mapping.".to_string())?;
        for (name, arr0) in arr_arr.iter() {
            let arr = arr0
                .as_object()
                .ok_or_else(|| format!("entry for array {} is not a mapping.", name))?;
            if !arr.contains_key("a") {
                return Err(format!("array {} does not have required field 'a'", name));
            }
            let a = arr
                .get("a")
                .unwrap()
                .as_array()
                .ok_or_else(|| format!("field 'a' of array {} is not an array.", name))?;
            if !a.is_empty() && a.iter().all(|x| x.is_i64()) {
                arrays.insert(
                    name.clone(),
                    a.iter().map(|x| x.as_i64().unwrap()).collect::<Vec<_>>(),
                );
            } else if !a.is_empty() && a.iter().all(|x| x.is_string()) {
                string_arrays.insert(
                    name.clone(),
                    a.iter()
                        .map(|x| x.as_str().unwrap().to_string())
                        .collect::<Vec<_>>(),
                );
            }
        }
    } else {
        return Err("missing required field 'arrays'.".to_string());
    }
    if let Some(cons_json) = json.get("constraints") {
        let cons = cons_json
            .as_array()
            .ok_or_else(|| "'constraints' is not an array.".to_string())?;
        if cons.iter().any(|x| !x.is_object()) {
            return Err("all entries in 'constraints' must be mappings.".to_string());
        }
        for c0 in cons.iter() {
            let c = c0.as_object().unwrap();
            if let Some(id) = c.get("id").and_then(|s| s.as_str()) {
                let args = c.get("args").and_then(|x| x.as_array()).ok_or_else(|| {
                    "all entries in 'constraints' must contain array 'args'".to_string()
                })?;
                if id.starts_with("set_")
                    || id.starts_with("array_set_")
                    || id.starts_with("array_var_set_")
                {
                    return Err("Flatzinc not implemented error: set constraints are currently unsupported.".to_string());
                }
                if id.starts_with("float_")
                    || id.starts_with("array_float_")
                    || id.starts_with("array_var_float_")
                    || id == "int2float"
                {
                    return Err("Flatzinc not implemented error: float constraints are currently unsupported.".to_string());
                }
                if id.ends_with("_reif") && id != "bool_clause_reif" {
                    return Err("Flatzinc not implemented error: reified constraints are currently unsupported.".to_string());
                }
                let mut success = false;
                if id.starts_with("int_lin") || id.starts_with("bool_lin") {
                    if args.len() != 3 {
                        return Err(format!(
                            "constraint {} has {} arguments instead of 3.",
                            id,
                            args.len()
                        ));
                    }
                    let arr = int_array_or_ref(&args[0], &arrays)
                        .map_err(|s| format!("coefficient array of constraint {}: {}", id, s))?;
                    let cvars = var_array_or_ref(&args[1], &string_arrays, &vars)
                        .map_err(|s| format!("variable array of constraint {}: {}", id, s))?;
                    let bound = args[2].as_i64().ok_or_else(|| {
                        format!("non-integer third argument to constraint {}", id)
                    })?;
                    match id {
                        "int_lin_eq" | "bool_lin_eq" => {
                            success = true;
                            solver.add_constraint(Box::new(LinearInequalityConstraint::new(
                                cvars.clone(),
                                arr.clone(),
                                bound,
                            )));
                            solver.add_constraint(Box::new(LinearInequalityConstraint::new(
                                cvars,
                                arr.into_iter().map(|x| -x).collect::<Vec<_>>(),
                                -bound,
                            )));
                        }
                        "int_lin_le" | "bool_lin_le" => {
                            success = true;
                            solver.add_constraint(Box::new(LinearInequalityConstraint::new(
                                cvars,
                                arr.clone(),
                                bound,
                            )));
                        }
                        "int_lin_ne" => {
                            return Err("Flatzinc not implemented error: int_lin_ne constraint is currently not supported, sorry.".to_string());
                        }
                        _ => {
                            return Err(format!("unknown linear constraint {}", id));
                        }
                    }
                }
                if !success {
                    match id {
                        "ezcp_alldifferent" => {
                            if args.len() != 1 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 2.",
                                    id,
                                    args.len()
                                ));
                            }
                            let cvars =
                                var_array_or_ref(&args[0], &string_arrays, &vars).map_err(|s| {
                                    format!("variable array of constraint {}: {}", id, s)
                                })?;
                            solver.add_constraint(Box::new(AllDifferentConstraint::new(cvars)));
                        }
                        "ezcp_bin_packing" => {
                            if args.len() != 3 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 3.",
                                    id,
                                    args.len()
                                ));
                            }
                            let cvars0 = var_array_or_ref(&args[0], &string_arrays, &vars)
                                .map_err(|s| {
                                    format!("load variables of constraint {}: {}", id, s)
                                })?;
                            let cvars1 = var_array_or_ref(&args[1], &string_arrays, &vars)
                                .map_err(|s| {
                                    format!("bin variables of constraint {}: {}", id, s)
                                })?;
                            let w = int_array_or_ref(&args[2], &arrays)
                                .map_err(|s| format!("weight array of constraint {}: {}", id, s))?;
                            solver.add_constraint(Box::new(BinPackingConstraint::new(
                                cvars1, cvars0, w,
                            )));
                        }
                        "int_eq" | "bool_eq" | "bool2int" => {
                            if args.len() != 2 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 2.",
                                    id,
                                    args.len()
                                ));
                            }
                            let cvars = var_array(args, &vars)
                                .map_err(|s| format!("variables of constraint {}: {}", id, s))?;
                            solver.add_constraint(Box::new(LinearInequalityConstraint::new(
                                cvars.clone(),
                                vec![1, -1],
                                0,
                            )));
                            solver.add_constraint(Box::new(LinearInequalityConstraint::new(
                                cvars,
                                vec![-1, 1],
                                0,
                            )));
                        }
                        "int_le" | "bool_le" => {
                            if args.len() != 2 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 2.",
                                    id,
                                    args.len()
                                ));
                            }
                            let cvars = var_array(args, &vars)
                                .map_err(|s| format!("variables of constraint {}: {}", id, s))?;
                            solver.add_constraint(Box::new(LinearInequalityConstraint::new(
                                cvars,
                                vec![1, -1],
                                0,
                            )));
                        }
                        "int_lt" | "bool_lt" => {
                            if args.len() != 2 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 2.",
                                    id,
                                    args.len()
                                ));
                            }
                            let cvars = var_array(args, &vars)
                                .map_err(|s| format!("variables of constraint {}: {}", id, s))?;
                            solver.add_constraint(Box::new(LinearInequalityConstraint::new(
                                cvars,
                                vec![1, -1],
                                -1,
                            )));
                        }
                        "int_plus" => {
                            if args.len() != 3 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 3.",
                                    id,
                                    args.len()
                                ));
                            }
                            let cvars = var_array(args, &vars)
                                .map_err(|s| format!("variables of constraint {}: {}", id, s))?;
                            solver.add_constraint(Box::new(LinearInequalityConstraint::new(
                                cvars.clone(),
                                vec![1, 1, -1],
                                0,
                            )));
                            solver.add_constraint(Box::new(LinearInequalityConstraint::new(
                                cvars,
                                vec![-1, -1, 1],
                                0,
                            )));
                        }
                        "bool_not" => {
                            if args.len() != 2 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 2.",
                                    id,
                                    args.len()
                                ));
                            }
                            let cvars = var_array(args, &vars)
                                .map_err(|s| format!("variables of constraint {}: {}", id, s))?;
                            solver.add_constraint(Box::new(NegateConstraint::new(
                                cvars[0].clone(),
                                cvars[1].clone(),
                            )));
                        }
                        "bool_and" => {
                            if args.len() != 3 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 3.",
                                    id,
                                    args.len()
                                ));
                            }
                            let cvars = var_array(args, &vars)
                                .map_err(|s| format!("variables of constraint {}: {}", id, s))?;
                            solver.add_constraint(Box::new(AndConstraint::new(
                                cvars[2].clone(),
                                cvars[..2].to_vec(),
                            )));
                        }
                        "bool_or" => {
                            if args.len() != 3 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 3.",
                                    id,
                                    args.len()
                                ));
                            }
                            let cvars = var_array(args, &vars)
                                .map_err(|s| format!("variables of constraint {}: {}", id, s))?;
                            solver.add_constraint(Box::new(OrConstraint::new(
                                cvars[2].clone(),
                                cvars[..2].to_vec(),
                            )));
                        }
                        "bool_clause" | "bool_clause_reif" => {
                            let need_args = if id == "bool_clause" { 2 } else { 3 };
                            if args.len() != need_args {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of {}.",
                                    id,
                                    args.len(),
                                    need_args
                                ));
                            }
                            let cvars0 = var_array_or_ref(&args[0], &string_arrays, &vars)
                                .map_err(|s| format!("variables of constraint {}: {}", id, s))?;
                            let cvars1 = var_array_or_ref(&args[1], &string_arrays, &vars)
                                .map_err(|s| format!("variables of constraint {}: {}", id, s))?;
                            let mut cvars2 = Vec::with_capacity(cvars0.len() + cvars1.len());
                            cvars2.extend_from_slice(&cvars0);
                            for v in &cvars1 {
                                let u =
                                    solver.new_variable(0, 1, format!("{}\tneg", &v.borrow().name));
                                solver.add_constraint(Box::new(NegateConstraint::new(
                                    v.clone(),
                                    u.clone(),
                                )));
                                cvars2.push(u);
                            }
                            let reif = if id == "bool_clause" {
                                solver.new_variable(1, 1, "alwaysone".to_string())
                            } else {
                                let varname = args[2].as_str().ok_or_else(|| {
                                    format!(
                                        "reified variable name for constraint {} is not a string",
                                        id
                                    )
                                })?;
                                vars.get(varname).cloned().ok_or_else(|| {
                                    format!("{} constraint has unknown variable {}.", id, varname)
                                })?
                            };
                            solver.add_constraint(Box::new(OrConstraint::new(reif, cvars2)));
                        }
                        _ => {
                            return Err(format!("Flatzinc not implemented error: no implementation for constraint {}", id));
                        }
                    }
                }
            } else {
                return Err("all entries in 'constraints' must contain string 'id'.".to_string());
            }
        }
    } else {
        return Err("missing required field 'constraints'.".to_string());
    }
    if let Some(out_json) = json.get("output") {
        let out = out_json
            .as_array()
            .ok_or_else(|| "'output' field is not an array of strings.".to_string())?;
        if out.iter().any(|x| !x.is_string()) {
            return Err("'output' field is not an array of strings.".to_string());
        }
        output = out
            .iter()
            .map(|x| x.as_str().unwrap().to_string())
            .collect::<Vec<String>>();
        if let Some(var) = output.iter().find(|s| !vars.contains_key(s.as_str())) {
            return Err(format!(
                "Output variable {} does not exist or has unsupported type.",
                var
            ));
        }
    } else {
        return Err("missing required field 'output'.".to_string());
    }
    if let Some(sol_json) = json.get("solve") {
        // we ignore solve annotations for now
        let method = sol_json
            .get("method")
            .and_then(|x| x.as_str())
            .ok_or_else(|| {
                "'solve' field does not contain 'method' or it is not a string.".to_string()
            })?;
        if method != "satisfy" {
            let obj = sol_json.get("objective").and_then(|x| x.as_str()).ok_or_else(|| "'objective' is not a string. Note: currently we only support variable names as objective.".to_string())?;
            if !vars.contains_key(obj) {
                return Err("'objective' is not a valid variable name. Note: currently we only support variable names as objective.".to_string());
            }
            let var = vars.get(obj).unwrap().clone();
            match method {
                "minimize" => {
                    solver.add_objective(Box::new(SingleVariableObjective { var, coeff: 1 }));
                }
                "maximize" => {
                    solver.add_objective(Box::new(SingleVariableObjective { var, coeff: -1 }));
                }
                _ => {
                    return Err(format!("unknown solve method {}", method));
                }
            }
        }
    } else {
        return Err("missing required field 'solve'.".to_string());
    }
    Ok(MinizincParseResult { solver, output })
}
