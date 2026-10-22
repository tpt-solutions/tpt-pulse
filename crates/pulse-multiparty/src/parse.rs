//! The text format for protocols. See `crates/pulse-cli/README.md` for the grammar.

use std::fmt;

use crate::{Statement, Step};

/// Words that start a statement and cannot be role names.
const KEYWORDS: [&str; 5] = ["loop", "choice", "by", "or", "continue"];

/// Text that could not be parsed as a protocol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError {
    /// The 1-based line where the problem was found.
    pub line: usize,
    /// What is wrong.
    pub reason: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "line {}: {}", self.line, self.reason)
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Word(String),
    Arrow,
    Colon,
    Semi,
    Open,
    Close,
}

/// Parse a whole protocol into its statements.
pub(crate) fn parse(text: &str) -> Result<Vec<Statement>, ParseError> {
    let tokens = tokenize(text)?;
    if tokens.is_empty() {
        return Err(ParseError {
            line: 1,
            reason: "the protocol has no steps".into(),
        });
    }
    let mut parser = Parser { tokens, pos: 0 };
    parser.block(false)
}

fn tokenize(text: &str) -> Result<Vec<(usize, Token)>, ParseError> {
    let mut tokens = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line_number = index + 1;
        let code = line.split("//").next().unwrap_or("");
        let mut word = String::new();
        let mut chars = code.chars().peekable();
        while let Some(c) = chars.next() {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            flush_word(&mut word, line_number, &mut tokens);
            let token = match c {
                c if c.is_whitespace() => continue,
                '-' if chars.peek() == Some(&'>') => {
                    chars.next();
                    Token::Arrow
                }
                ':' => Token::Colon,
                ';' => Token::Semi,
                '{' => Token::Open,
                '}' => Token::Close,
                other => {
                    return Err(ParseError {
                        line: line_number,
                        reason: format!("unexpected character `{other}`"),
                    })
                }
            };
            tokens.push((line_number, token));
        }
        flush_word(&mut word, line_number, &mut tokens);
    }
    Ok(tokens)
}

fn flush_word(word: &mut String, line: usize, tokens: &mut Vec<(usize, Token)>) {
    if !word.is_empty() {
        tokens.push((line, Token::Word(std::mem::take(word))));
    }
}

struct Parser {
    tokens: Vec<(usize, Token)>,
    pos: usize,
}

impl Parser {
    fn line(&self) -> usize {
        self.tokens
            .get(self.pos)
            .or(self.tokens.last())
            .map_or(1, |(line, _)| *line)
    }

    fn error<T>(&self, reason: impl Into<String>) -> Result<T, ParseError> {
        Err(ParseError {
            line: self.line(),
            reason: reason.into(),
        })
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos).map(|(_, token)| token)
    }

    fn peek_word(&self, expected: &str) -> bool {
        matches!(self.peek(), Some(Token::Word(word)) if word == expected)
    }

    fn expect(&mut self, wanted: &Token, what: &str) -> Result<(), ParseError> {
        if self.peek() == Some(wanted) {
            self.pos += 1;
            Ok(())
        } else {
            self.error(format!("expected {what}"))
        }
    }

    fn keyword(&mut self, expected: &str) -> Result<(), ParseError> {
        if self.peek_word(expected) {
            self.pos += 1;
            Ok(())
        } else {
            self.error(format!("expected `{expected}`"))
        }
    }

    fn word(&mut self, what: &str) -> Result<String, ParseError> {
        match self.peek() {
            Some(Token::Word(word)) => {
                let word = word.clone();
                self.pos += 1;
                Ok(word)
            }
            _ => self.error(format!("expected {what}")),
        }
    }

    fn role(&mut self) -> Result<String, ParseError> {
        let name = self.word("a role name")?;
        if KEYWORDS.contains(&name.as_str()) {
            return self.error(format!("`{name}` is a keyword, not a role name"));
        }
        Ok(name)
    }

    /// Parse statements until the end of input, or until the `}` that closes a block.
    fn block(&mut self, nested: bool) -> Result<Vec<Statement>, ParseError> {
        let mut statements = Vec::new();
        loop {
            match self.peek().cloned() {
                None if nested => return self.error("unclosed `{`"),
                None => return Ok(statements),
                Some(Token::Close) if nested => {
                    self.pos += 1;
                    return Ok(statements);
                }
                Some(Token::Close) => return self.error("unexpected `}`"),
                Some(Token::Word(word)) if word == "loop" => {
                    self.pos += 1;
                    self.expect(&Token::Open, "`{` after `loop`")?;
                    let body = self.block(true)?;
                    statements.push(Statement::Loop(body));
                }
                Some(Token::Word(word)) if word == "choice" => {
                    self.pos += 1;
                    self.keyword("by")?;
                    let decider = self.role()?;
                    self.expect(&Token::Open, "`{` after the decider")?;
                    let mut branches = vec![self.block(true)?];
                    while self.peek_word("or") {
                        self.pos += 1;
                        self.expect(&Token::Open, "`{` after `or`")?;
                        branches.push(self.block(true)?);
                    }
                    statements.push(Statement::Choice { decider, branches });
                }
                Some(Token::Word(word)) if word == "continue" => {
                    self.pos += 1;
                    self.expect(&Token::Semi, "`;` after `continue`")?;
                    statements.push(Statement::Continue);
                }
                Some(Token::Word(_)) => {
                    let from = self.role()?;
                    self.expect(&Token::Arrow, "`->` after the sender")?;
                    let to = self.role()?;
                    self.expect(&Token::Colon, "`:` before the message label")?;
                    let label = self.word("a message label")?;
                    self.expect(&Token::Semi, "`;` after a step")?;
                    statements.push(Statement::Message(Step { from, to, label }));
                }
                Some(_) => return self.error("expected a step"),
            }
        }
    }
}
