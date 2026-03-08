/// Private token type produced by [`tokenize`].
#[derive(Debug, PartialEq, Clone)]
pub(crate) enum Token {
    Word(String),
    /// `--name` or `--name=value`
    LongFlag { name: String, value: Option<String> },
    /// `-f` or `-fvalue`
    ShortFlag { name: char, value: Option<String> },
    /// Bare `--` separator
    Separator,
}

/// Tokenize a raw argument slice into typed tokens.
///
/// After a bare `--` separator all subsequent arguments become [`Token::Word`].
pub(crate) fn tokenize(argv: &[&str]) -> Vec<Token> {
    let mut tokens = Vec::with_capacity(argv.len());
    let mut after_sep = false;

    for arg in argv {
        if after_sep {
            tokens.push(Token::Word(arg.to_string()));
            continue;
        }

        if *arg == "--" {
            tokens.push(Token::Separator);
            after_sep = true;
            continue;
        }

        if let Some(rest) = arg.strip_prefix("--") {
            if let Some(eq) = rest.find('=') {
                tokens.push(Token::LongFlag {
                    name: rest[..eq].to_string(),
                    value: Some(rest[eq + 1..].to_string()),
                });
            } else {
                tokens.push(Token::LongFlag {
                    name: rest.to_string(),
                    value: None,
                });
            }
            continue;
        }

        if let Some(rest) = arg.strip_prefix('-') {
            if let Some(c) = rest.chars().next() {
                let remainder = &rest[c.len_utf8()..];
                let value = if remainder.is_empty() {
                    None
                } else {
                    Some(remainder.to_string())
                };
                tokens.push(Token::ShortFlag { name: c, value });
            } else {
                // bare `-` treated as a word
                tokens.push(Token::Word(arg.to_string()));
            }
            continue;
        }

        tokens.push(Token::Word(arg.to_string()));
    }

    tokens
}

#[cfg(test)]
mod tests {
    use super::*;

    struct TestCase {
        name: &'static str,
        input: &'static [&'static str],
        expected: Vec<Token>,
    }

    #[test]
    fn test_tokenize() {
        let cases = vec![
            TestCase {
                name: "words only",
                input: &["git", "status"],
                expected: vec![
                    Token::Word("git".into()),
                    Token::Word("status".into()),
                ],
            },
            TestCase {
                name: "long flag no value",
                input: &["--verbose"],
                expected: vec![Token::LongFlag {
                    name: "verbose".into(),
                    value: None,
                }],
            },
            TestCase {
                name: "long flag with equals",
                input: &["--output=file.txt"],
                expected: vec![Token::LongFlag {
                    name: "output".into(),
                    value: Some("file.txt".into()),
                }],
            },
            TestCase {
                name: "short flag alone",
                input: &["-v"],
                expected: vec![Token::ShortFlag { name: 'v', value: None }],
            },
            TestCase {
                name: "short flag with value",
                input: &["-ofile.txt"],
                expected: vec![Token::ShortFlag {
                    name: 'o',
                    value: Some("file.txt".into()),
                }],
            },
            TestCase {
                name: "separator",
                input: &["cmd", "--", "--not-a-flag"],
                expected: vec![
                    Token::Word("cmd".into()),
                    Token::Separator,
                    Token::Word("--not-a-flag".into()),
                ],
            },
            TestCase {
                name: "bare dash is word",
                input: &["-"],
                expected: vec![Token::Word("-".into())],
            },
        ];

        for tc in &cases {
            let got = tokenize(tc.input);
            assert_eq!(got, tc.expected, "case: {}", tc.name);
        }
    }
}
