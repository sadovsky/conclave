use conclave_lang::lexer::{tokenize, Token};

fn tok(input: &str) -> Vec<Token> {
    tokenize(input)
        .unwrap()
        .into_iter()
        .map(|s| s.token)
        .collect()
}

#[test]
fn goal_ident_parens() {
    assert_eq!(
        tok("goal Foo()"),
        vec![
            Token::Goal,
            Token::Ident("Foo".into()),
            Token::LParen,
            Token::RParen
        ]
    );
}

#[test]
fn req_per_sec_unit() {
    assert_eq!(tok("2 req/s"), vec![Token::Number(2), Token::ReqPerSec]);
}

#[test]
fn req_per_sec_no_space() {
    assert_eq!(tok("2req/s"), vec![Token::Number(2), Token::ReqPerSec]);
}

#[test]
fn arrow_token() {
    assert_eq!(
        tok("-> Html"),
        vec![Token::Arrow, Token::Ident("Html".into())]
    );
}

#[test]
fn lteq_token() {
    assert_eq!(tok("<= 2"), vec![Token::LtEq, Token::Number(2)]);
}

#[test]
fn version_line() {
    assert_eq!(
        tok("version 0.1;"),
        vec![
            Token::Version,
            Token::Number(0),
            Token::Dot,
            Token::Number(1),
            Token::Semicolon
        ]
    );
}

#[test]
fn string_literal() {
    assert_eq!(tok(r#""hello""#), vec![Token::StringLit("hello".into())]);
}

#[test]
fn line_comment_skipped() {
    assert_eq!(tok("// this is a comment\ngoal"), vec![Token::Goal]);
}

#[test]
fn keywords_recognized() {
    let kws =
        "version type capability intrinsic goal want map as let emit return constraints where";
    let tokens = tok(kws);
    assert_eq!(
        tokens,
        vec![
            Token::Version,
            Token::Type,
            Token::Capability,
            Token::Intrinsic,
            Token::Goal,
            Token::Want,
            Token::Map,
            Token::As,
            Token::Let,
            Token::Emit,
            Token::Return,
            Token::Constraints,
            Token::Where,
        ]
    );
}

#[test]
fn unknown_char_errors() {
    let result = tokenize("@");
    assert!(result.is_err());
}

#[test]
fn unterminated_string_errors() {
    let result = tokenize(r#""unterminated"#);
    assert!(result.is_err());
}
