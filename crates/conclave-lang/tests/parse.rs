use conclave_lang::ast::*;
use conclave_lang::parser::parse;

fn source() -> &'static str {
    include_str!("fixtures/summarize_urls/source.conclave")
}

#[test]
fn parse_version() {
    let m = parse(source()).unwrap();
    assert_eq!(m.version, "0.1");
}

#[test]
fn parse_type_decl() {
    let m = parse(source()).unwrap();
    assert_eq!(m.types.len(), 1);
    let t = &m.types[0];
    assert_eq!(t.name, "Url");
    assert_eq!(t.base, "String");
    let c = t.constraint.as_ref().unwrap();
    assert_eq!(c.validator, "re2");
    assert_eq!(c.pattern, "^https?://");
}

#[test]
fn parse_capabilities() {
    let m = parse(source()).unwrap();
    assert_eq!(m.capabilities.len(), 3);
    let names: Vec<&str> = m.capabilities.iter().map(|c| c.alias.as_str()).collect();
    assert!(names.contains(&"fetch"));
    assert!(names.contains(&"extract_text"));
    assert!(names.contains(&"summarize"));
}

#[test]
fn parse_intrinsic() {
    let m = parse(source()).unwrap();
    assert_eq!(m.intrinsics.len(), 1);
    assert_eq!(m.intrinsics[0].alias, "assemble_json");
}

#[test]
fn parse_goal_declaration() {
    let m = parse(source()).unwrap();
    assert_eq!(m.goals.len(), 1);
    let g = &m.goals[0];
    assert_eq!(g.name, "SummarizeUrls");
    assert_eq!(g.returns, "Json");
    assert_eq!(g.params.len(), 1);
    assert_eq!(g.params[0].name, "urls");
    assert_eq!(g.params[0].type_name, "List<Url>");
}

#[test]
fn parse_want_map_structure() {
    let m = parse(source()).unwrap();
    let want = &m.goals[0].want;
    assert_eq!(want.stmts.len(), 2); // map + return
    match &want.stmts[0] {
        Stmt::Map { list, binder, body } => {
            assert_eq!(list, "urls");
            assert_eq!(binder, "url");
            assert_eq!(body.len(), 3); // let html, let text, emit
        }
        _ => panic!("expected Map statement"),
    }
}

#[test]
fn parse_let_statements() {
    let m = parse(source()).unwrap();
    let body = match &m.goals[0].want.stmts[0] {
        Stmt::Map { body, .. } => body,
        _ => panic!("expected map"),
    };
    match &body[0] {
        Stmt::Let { name, expr } => {
            assert_eq!(name, "html");
            match expr {
                Expr::Call { name, args } => {
                    assert_eq!(name, "fetch");
                    assert_eq!(args.len(), 1);
                    match &args[0] {
                        Expr::Ident { name } => assert_eq!(name, "url"),
                        _ => panic!("expected ident"),
                    }
                }
                _ => panic!("expected call"),
            }
        }
        _ => panic!("expected let"),
    }
}

#[test]
fn parse_return_statement() {
    let m = parse(source()).unwrap();
    let want = &m.goals[0].want;
    match want.stmts.last().unwrap() {
        Stmt::Return { expr } => match expr {
            Expr::Call { name, args } => {
                assert_eq!(name, "assemble_json");
                assert_eq!(args.len(), 1);
                match &args[0] {
                    Expr::Ident { name } => assert_eq!(name, "collected"),
                    _ => panic!("expected 'collected' ident"),
                }
            }
            _ => panic!("expected call"),
        },
        _ => panic!("expected return"),
    }
}

#[test]
fn parse_constraints() {
    let m = parse(source()).unwrap();
    let cs = &m.goals[0].constraints;
    assert_eq!(cs.len(), 3);
    // determinism.mode == "sealed_replay"
    match &cs[0] {
        c if matches!(c.op, CmpOp::Eq) => {
            match &c.left {
                ConstraintLeft::Path { segments } => {
                    assert_eq!(segments, &["determinism", "mode"]);
                }
                _ => panic!("expected path"),
            }
            match &c.right {
                ConstraintValue::StringLit { value } => assert_eq!(value, "sealed_replay"),
                _ => panic!("expected string"),
            }
        }
        _ => panic!("expected == constraint"),
    }
    // rate_limit(fetch) <= 2 req/s
    match &cs[1] {
        c if matches!(c.op, CmpOp::LtEq) => {
            match &c.left {
                ConstraintLeft::FnCall { name, args } => {
                    assert_eq!(name, "rate_limit");
                    assert_eq!(args, &["fetch"]);
                }
                _ => panic!("expected fn call"),
            }
            match &c.right {
                ConstraintValue::Rate { value, unit } => {
                    assert_eq!(*value, 2);
                    assert_eq!(unit, "req/s");
                }
                _ => panic!("expected rate"),
            }
        }
        _ => panic!("expected <= constraint"),
    }
}

#[test]
fn parse_error_unknown_token() {
    let result = parse("version 0.1;\n@garbage");
    assert!(result.is_err());
}

#[test]
fn parse_error_missing_version() {
    let result = parse("goal Foo() -> Json { want { return assemble_json(collected); } }");
    assert!(result.is_err());
}
