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
use std::collections::{HashMap, HashSet};
use std::rc::Rc;

pub struct MinizincParseResult {
    pub solver: Solver,
    pub output: Vec<String>,
}

pub fn parse(json: serde_json::Value) -> Result<MinizincParseResult, String> {
    let mut solver = Solver::new(Config::new(
        Box::new(MinValueSelector {}),
        Box::new(FirstFailVariableSelector {}),
    ));
    let mut vars = HashMap::<String, Rc<RefCell<Variable>>>::new();
    let mut arrays = HashMap::<String, Vec<i64>>::new();
    let mut skipped_arrays = HashSet::<String>::new();
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
            if a.is_empty() || a.iter().any(|x| !x.is_i64()) {
                skipped_arrays.insert(name.clone());
                continue;
            }
            arrays.insert(
                name.clone(),
                a.iter().map(|x| x.as_i64().unwrap()).collect::<Vec<_>>(),
            );
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
                if id.ends_with("_reif") {
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
                    let arr = if let Some(arr_id) = args[0].as_str() {
                        if skipped_arrays.contains(arr_id) {
                            return Err(format!(
                                "array {} used in {} constraint is not integer.",
                                arr_id, id
                            ));
                        }
                        if !arrays.contains_key(arr_id) {
                            return Err(format!(
                                "{} constraint uses unknown array {}.",
                                id, arr_id
                            ));
                        }
                        arrays.get(arr_id).unwrap().to_vec()
                    } else if let Some(arr) = args[0].as_array() {
                        if arr.iter().any(|a| !a.is_i64()) {
                            return Err(format!(
                                "coefficient array of constraint {} contains non-integer values.",
                                id
                            ));
                        }
                        arr.iter()
                            .map(|a| a.as_i64().unwrap())
                            .collect::<Vec<i64>>()
                    } else {
                        return Err(format!(
                            "first argument to constraint {} is not a string or array",
                            id
                        ));
                    };
                    let varnames = args[1]
                        .as_array()
                        .and_then(|a| {
                            if a.iter().any(|x| !x.is_string()) {
                                None
                            } else {
                                Some(a.iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>())
                            }
                        })
                        .ok_or_else(|| {
                            format!(
                                "second argument to constraint {} is not an array of strings.",
                                id
                            )
                        })?;
                    if let Some(s) = varnames.iter().find(|s| !vars.contains_key(**s)) {
                        return Err(format!("{} constraint has unknown variable {}.", id, s));
                    }
                    let cvars = varnames
                        .iter()
                        .map(|s| vars.get(*s).unwrap().clone())
                        .collect::<Vec<_>>();
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
                                arr.iter().map(|x| -*x).collect::<Vec<_>>(),
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
                        "int_eq" | "bool_eq" | "bool2int" => {
                            if args.len() != 2 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 2.",
                                    id,
                                    args.len()
                                ));
                            }
                            if args.iter().any(|x| !x.is_string()) {
                                return Err(format!(
                                    "{} constraint has non-string variable name!",
                                    id
                                ));
                            }
                            let varnames =
                                args.iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>();
                            if let Some(s) = varnames.iter().find(|s| !vars.contains_key(**s)) {
                                return Err(format!(
                                    "{} constraint has unknown variable {}.",
                                    id, s
                                ));
                            }
                            let cvars = varnames
                                .iter()
                                .map(|s| vars.get(*s).unwrap().clone())
                                .collect::<Vec<_>>();
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
                            if args.iter().any(|x| !x.is_string()) {
                                return Err(format!(
                                    "{} constraint has non-string variable name!",
                                    id
                                ));
                            }
                            let varnames =
                                args.iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>();
                            if let Some(s) = varnames.iter().find(|s| !vars.contains_key(**s)) {
                                return Err(format!(
                                    "{} constraint has unknown variable {}.",
                                    id, s
                                ));
                            }
                            let cvars = varnames
                                .iter()
                                .map(|s| vars.get(*s).unwrap().clone())
                                .collect::<Vec<_>>();
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
                            if args.iter().any(|x| !x.is_string()) {
                                return Err(format!(
                                    "{} constraint has non-string variable name!",
                                    id
                                ));
                            }
                            let varnames =
                                args.iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>();
                            if let Some(s) = varnames.iter().find(|s| !vars.contains_key(**s)) {
                                return Err(format!(
                                    "{} constraint has unknown variable {}.",
                                    id, s
                                ));
                            }
                            let cvars = varnames
                                .iter()
                                .map(|s| vars.get(*s).unwrap().clone())
                                .collect::<Vec<_>>();
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
                            if args.iter().any(|x| !x.is_string()) {
                                return Err(format!(
                                    "{} constraint has non-string variable name!",
                                    id
                                ));
                            }
                            let varnames =
                                args.iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>();
                            if let Some(s) = varnames.iter().find(|s| !vars.contains_key(**s)) {
                                return Err(format!(
                                    "{} constraint has unknown variable {}.",
                                    id, s
                                ));
                            }
                            let cvars = varnames
                                .iter()
                                .map(|s| vars.get(*s).unwrap().clone())
                                .collect::<Vec<_>>();
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
                            if args.iter().any(|x| !x.is_string()) {
                                return Err(format!(
                                    "{} constraint has non-string variable name!",
                                    id
                                ));
                            }
                            let varnames =
                                args.iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>();
                            if let Some(s) = varnames.iter().find(|s| !vars.contains_key(**s)) {
                                return Err(format!(
                                    "{} constraint has unknown variable {}.",
                                    id, s
                                ));
                            }
                            let cvars = varnames
                                .iter()
                                .map(|s| vars.get(*s).unwrap().clone())
                                .collect::<Vec<_>>();
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
                            if args.iter().any(|x| !x.is_string()) {
                                return Err(format!(
                                    "{} constraint has non-string variable name!",
                                    id
                                ));
                            }
                            let varnames =
                                args.iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>();
                            if let Some(s) = varnames.iter().find(|s| !vars.contains_key(**s)) {
                                return Err(format!(
                                    "{} constraint has unknown variable {}.",
                                    id, s
                                ));
                            }
                            let cvars = varnames
                                .iter()
                                .map(|s| vars.get(*s).unwrap().clone())
                                .collect::<Vec<_>>();
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
                            if args.iter().any(|x| !x.is_string()) {
                                return Err(format!(
                                    "{} constraint has non-string variable name!",
                                    id
                                ));
                            }
                            let varnames =
                                args.iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>();
                            if let Some(s) = varnames.iter().find(|s| !vars.contains_key(**s)) {
                                return Err(format!(
                                    "{} constraint has unknown variable {}.",
                                    id, s
                                ));
                            }
                            let cvars = varnames
                                .iter()
                                .map(|s| vars.get(*s).unwrap().clone())
                                .collect::<Vec<_>>();
                            solver.add_constraint(Box::new(OrConstraint::new(
                                cvars[2].clone(),
                                cvars[..2].to_vec(),
                            )));
                        }
                        "bool_clause" => {
                            if args.len() != 2 {
                                return Err(format!(
                                    "constraint {} has {} arguments instead of 2.",
                                    id,
                                    args.len()
                                ));
                            }
                            let varnames0 = args[0].as_array().and_then(|a| if a.iter().any(|x| !x.is_string()) { None } else { Some(a.iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>()) }).ok_or_else(|| format!("second argument to constraint {} is not an array of strings.", id))?;
                            if let Some(s) = varnames0.iter().find(|s| !vars.contains_key(**s)) {
                                return Err(format!(
                                    "{} constraint has unknown variable {}.",
                                    id, s
                                ));
                            }
                            let cvars0 = varnames0
                                .iter()
                                .map(|s| vars.get(*s).unwrap().clone())
                                .collect::<Vec<_>>();
                            let varnames1 = args[1].as_array().and_then(|a| if a.iter().any(|x| !x.is_string()) { None } else { Some(a.iter().map(|x| x.as_str().unwrap()).collect::<Vec<&str>>()) }).ok_or_else(|| format!("second argument to constraint {} is not an array of strings.", id))?;
                            if let Some(s) = varnames1.iter().find(|s| !vars.contains_key(**s)) {
                                return Err(format!(
                                    "{} constraint has unknown variable {}.",
                                    id, s
                                ));
                            }
                            let cvars1 = varnames1
                                .iter()
                                .map(|s| vars.get(*s).unwrap().clone())
                                .collect::<Vec<_>>();
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
                            let alwaysone = solver.new_variable(1, 1, "alwaysone".to_string());
                            solver.add_constraint(Box::new(OrConstraint::new(alwaysone, cvars2)));
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
