use crate::error::LangError;

/// A token produced by the Conclave v0.1 lexer.
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // --- Keywords ---
    Version,
    Import,
    Type,
    Capability,
    Intrinsic,
    Goal,
    Want,
    Map,
    As,
    Let,
    Emit,
    Return,
    Constraints,
    Where,

    // --- Punctuation ---
    LBrace,
    RBrace,
    LParen,
    RParen,
    LAngle, // <
    RAngle, // >
    Semicolon,
    Comma,
    Colon,
    Equals,
    Arrow, // ->
    LtEq,  // <=
    Dot,   // .

    // --- Literals ---
    Ident(String),
    StringLit(String),
    /// Integer literal (no leading zeros except the literal "0").
    Number(u64),

    // --- Units ---
    ReqPerSec, // req/s (lexed as a unit token after a Number)
}

/// A token together with the 1-based source line it starts on.
#[derive(Debug, Clone)]
pub struct Spanned {
    pub token: Token,
    pub line: usize,
}

/// Tokenize a Conclave v0.1 source string.
///
/// Line endings are expected to already be normalized to LF (`\n`).
/// Returns a flat `Vec<Spanned>` or a `LangError` on the first bad character.
pub fn tokenize(input: &str) -> Result<Vec<Spanned>, LangError> {
    let bytes = input.as_bytes();
    let len = bytes.len();
    let mut pos = 0usize;
    let mut line = 1usize;
    let mut tokens = Vec::new();

    while pos < len {
        // Skip whitespace.
        if bytes[pos] == b' ' || bytes[pos] == b'\t' || bytes[pos] == b'\r' {
            pos += 1;
            continue;
        }
        if bytes[pos] == b'\n' {
            line += 1;
            pos += 1;
            continue;
        }

        // Skip `//` line comments.
        if pos + 1 < len && bytes[pos] == b'/' && bytes[pos + 1] == b'/' {
            while pos < len && bytes[pos] != b'\n' {
                pos += 1;
            }
            continue;
        }

        // Two-character tokens first.
        if pos + 1 < len {
            match (bytes[pos], bytes[pos + 1]) {
                (b'-', b'>') => {
                    tokens.push(Spanned {
                        token: Token::Arrow,
                        line,
                    });
                    pos += 2;
                    continue;
                }
                (b'<', b'=') => {
                    tokens.push(Spanned {
                        token: Token::LtEq,
                        line,
                    });
                    pos += 2;
                    continue;
                }
                _ => {}
            }
        }

        // Single-character tokens.
        let single = match bytes[pos] {
            b'{' => Some(Token::LBrace),
            b'}' => Some(Token::RBrace),
            b'(' => Some(Token::LParen),
            b')' => Some(Token::RParen),
            b'<' => Some(Token::LAngle),
            b'>' => Some(Token::RAngle),
            b';' => Some(Token::Semicolon),
            b',' => Some(Token::Comma),
            b':' => Some(Token::Colon),
            b'=' => Some(Token::Equals),
            b'.' => Some(Token::Dot),
            _ => None,
        };
        if let Some(t) = single {
            tokens.push(Spanned { token: t, line });
            pos += 1;
            continue;
        }

        // String literals: "..."
        if bytes[pos] == b'"' {
            pos += 1; // skip opening quote
            let start = pos;
            while pos < len && bytes[pos] != b'"' {
                if bytes[pos] == b'\n' {
                    line += 1;
                }
                pos += 1;
            }
            if pos >= len {
                return Err(LangError::UnexpectedEof {
                    expected: "closing '\"'".into(),
                });
            }
            let s = &input[start..pos];
            tokens.push(Spanned {
                token: Token::StringLit(s.to_string()),
                line,
            });
            pos += 1; // skip closing quote
            continue;
        }

        // Number literals.
        if bytes[pos].is_ascii_digit() {
            let start = pos;
            while pos < len && bytes[pos].is_ascii_digit() {
                pos += 1;
            }
            let num_str = &input[start..pos];
            let num: u64 = num_str.parse().map_err(|_| LangError::UnexpectedToken {
                expected: "integer".into(),
                got: num_str.to_string(),
                line,
            })?;

            // Check for `req/s` immediately following (with optional whitespace).
            let tok_line = line;
            let mut lookahead = pos;
            while lookahead < len && (bytes[lookahead] == b' ' || bytes[lookahead] == b'\t') {
                lookahead += 1;
            }
            if lookahead + 4 < len && &input[lookahead..lookahead + 5] == "req/s" {
                // Check that nothing alphanumeric follows "req/s".
                let after = lookahead + 5;
                let next_is_alphanum =
                    after < len && (bytes[after].is_ascii_alphanumeric() || bytes[after] == b'_');
                if !next_is_alphanum {
                    tokens.push(Spanned {
                        token: Token::Number(num),
                        line: tok_line,
                    });
                    tokens.push(Spanned {
                        token: Token::ReqPerSec,
                        line: tok_line,
                    });
                    pos = after;
                    continue;
                }
            }

            tokens.push(Spanned {
                token: Token::Number(num),
                line: tok_line,
            });
            continue;
        }

        // Identifiers and keywords.
        if bytes[pos].is_ascii_alphabetic() || bytes[pos] == b'_' {
            let start = pos;
            while pos < len && (bytes[pos].is_ascii_alphanumeric() || bytes[pos] == b'_') {
                pos += 1;
            }
            let word = &input[start..pos];
            let tok = keyword_or_ident(word);
            tokens.push(Spanned { token: tok, line });
            continue;
        }

        return Err(LangError::UnexpectedToken {
            expected: "token".into(),
            got: (bytes[pos] as char).to_string(),
            line,
        });
    }

    Ok(tokens)
}

fn keyword_or_ident(word: &str) -> Token {
    match word {
        "version" => Token::Version,
        "import" => Token::Import,
        "type" => Token::Type,
        "capability" => Token::Capability,
        "intrinsic" => Token::Intrinsic,
        "goal" => Token::Goal,
        "want" => Token::Want,
        "map" => Token::Map,
        "as" => Token::As,
        "let" => Token::Let,
        "emit" => Token::Emit,
        "return" => Token::Return,
        "constraints" => Token::Constraints,
        "where" => Token::Where,
        _ => Token::Ident(word.to_string()),
    }
}
