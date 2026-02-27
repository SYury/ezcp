use assert_cmd::cargo;
use std::collections::HashMap;
use std::ffi::OsStr;
use std::fs::read_dir;
use std::process::Command;

#[test]
fn test_fzn_examples() {
    // it's bothersome to test all solutions for correctness, but at least we can test the objectives of optimization problems
    let test_specific_substrings = [
        ("warehouse.json".to_string(), "tot = 383;".to_string()),
        ("golomb.json".to_string(), ",17]".to_string()),
    ]
    .into_iter()
    .collect::<HashMap<String, String>>();
    let paths = read_dir("tests/fzn-examples").unwrap();
    for p in paths.filter(|pp| {
        if let Ok(p) = pp {
            p.path().is_file() && p.path().extension() == Some(OsStr::new("json"))
        } else {
            false
        }
    }) {
        let path = p.unwrap().path();
        let testname = path.file_name().unwrap().to_str().unwrap().to_string();
        eprintln!("testing fzn example {}", &testname);
        let mut cmd = Command::new(cargo::cargo_bin!("ezcp-fzn"));
        let out = cmd.arg(path).output().unwrap();
        assert!(out.status.success());
        if let Some(substr) = test_specific_substrings.get(&testname) {
            let txt = std::str::from_utf8(&out.stdout).unwrap();
            assert!(txt.contains(substr));
        }
    }
}
