#[test]
fn test_expr_0() {
    let out = cargo_script!("-e", with_output_marker!("0")).unwrap();
    scan!(out.stdout_output();
        ("0") => ()
    ).unwrap()
}

#[test]
fn test_expr_dnc() {
    let out = cargo_script!("-e", "swing begin").unwrap();
    assert!(!out.success());
}

#[test]
fn test_expr_dep() {
    let out = cargo_script!("-D", "boolinator=0.1.0",
        "-e", with_output_marker!(
            prelude "use boolinator::Boolinator;";
            "true.as_some(1)"
        )).unwrap();
    scan!(out.stdout_output();
        ("Some(1)") => ()
    ).unwrap()
}

#[test]
fn test_expr_dep_extern() {
    let out = cargo_script!("-d", "boolinator=0.1.0", "-x", "boolinator",
        "-e", with_output_marker!(
            prelude "use boolinator::Boolinator;";
            "true.as_some(1)"
        )).unwrap();
    scan!(out.stdout_output();
        ("Some(1)") => ()
    ).unwrap();

    let out = cargo_script!("-d", "boolinator=0.1.0",
        "-e", with_output_marker!(
            prelude "use boolinator::Boolinator;";
            "true.as_some(1)"
        )).unwrap();
    assert!(!out.success());

    let out = cargo_script!("-x", "boolinator",
        "-e", with_output_marker!("true")).unwrap();
    assert!(!out.success());

    let out = cargo_script!("-e", with_output_marker!(
        prelude "use boolinator::Boolinator;";
        "true.as_some(1)"
    )).unwrap();
    assert!(!out.success());
}

#[test]
fn test_expr_panic() {
    let out = cargo_script!("-e", with_output_marker!("panic!()")).unwrap();
    assert!(!out.success());
}

#[test]
fn test_expr_comma() {
    let out = cargo_script!("-e", with_output_marker!("(1, 2)")).unwrap();
    scan!(out.stdout_output();
        ("(1, 2)") => ()
    ).unwrap()
}
