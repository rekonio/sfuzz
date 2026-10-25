use regex::Regex;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

pub struct FuzzToken {
    pub kind: FuzzKind,
    happy_value: String,
    start: usize,
    end: usize,
}

#[derive(Debug, PartialEq)]
pub enum FuzzKind {
    StringKind,
    NumeralKind,
}

#[derive(Default)]
pub struct Tokenized {
    pub text: String,
    tokens: Vec<FuzzToken>,
}

impl Tokenized {
    pub fn new(bytes: Option<&str>) -> Self {
        match bytes {
            None => Tokenized::default(),
            Some(text) => Self {
                tokens: parse_tokens(text),
                text: text.to_string(),
            },
        }
    }

    pub fn size(&self) -> usize {
        self.tokens.len()
    }
    pub fn tokens(&self) -> impl Iterator<Item = &FuzzToken> {
        self.tokens.iter()
    }

    pub fn replace(&self, skip: Option<usize>) -> String {
        let mut text = self.text.clone();
        let mut done: usize = 0;
        for (i, tok) in self.tokens.iter().enumerate() {
            if skip == Some(i) {
                continue;
            }
            let offset = done * FUZZ_TOKEN_SIZE;
            let start = tok.start - offset;
            let end = tok.end - offset;
            text.replace_range(start..end, &tok.happy_value);
            done += 1;
        }
        text
    }
}

const FUZZ_TOKEN_SIZE: usize = 7;

fn parse_tokens(text: &str) -> Vec<FuzzToken> {
    let mut tokens = Vec::new();
    let re = Regex::new(r"FUZZ(.*?)(STR|NUM)").unwrap();
    for c in re.captures_iter(text) {
        let range = c.get(0).unwrap().range();
        let tok = FuzzToken {
            happy_value: c.get(1).unwrap().as_str().to_string(),
            kind: c.get(2).unwrap().as_str().parse().unwrap(),
            start: range.start,
            end: range.end,
        };
        tokens.push(tok);
    }
    tokens
}

impl Display for FuzzKind {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            FuzzKind::StringKind => write!(f, "string"),
            FuzzKind::NumeralKind => write!(f, "numeral"),
        }
    }
}

impl FromStr for FuzzKind {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "STR" => Ok(FuzzKind::StringKind),
            "NUM" => Ok(FuzzKind::NumeralKind),
            _ => Err("invalid fuzz kind".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::FuzzKind::{NumeralKind, StringKind};

    #[test]
    fn test_parse() {
        let text = "https://api.com/path/FUZZjohnSTR?age=FUZZ29NUM&q=FUZZSTR";
        let tokenized = Tokenized::new(Some(text));
        assert_eq!(tokenized.size(), 3);

        let mut it = tokenized.tokens();

        let first = it.next().unwrap();
        assert_eq!(first.kind, StringKind);
        assert_eq!(first.happy_value, "john");
        assert_eq!(first.start, 21);
        assert_eq!(first.end, 32);
        let second = it.next().unwrap();
        assert_eq!(second.kind, NumeralKind);
        assert_eq!(second.happy_value, "29");
        assert_eq!(second.start, 37);
        assert_eq!(second.end, 46);
        let third = it.next().unwrap();
        assert_eq!(third.kind, StringKind);
        assert_eq!(third.happy_value, "");
        assert_eq!(third.start, 49);
        assert_eq!(third.end, 56);
    }

    #[test]
    fn test_replace() {
        let text = "https://api.com/path/FUZZjohnSTR?age=FUZZ29NUM&q=FUZZSTR";
        let tokenized = Tokenized::new(Some(text));

        let result = tokenized.replace(None);
        assert_eq!(result, "https://api.com/path/john?age=29&q=");

        let result = tokenized.replace(Some(0));
        assert_eq!(result, "https://api.com/path/FUZZjohnSTR?age=29&q=");

        let result = tokenized.replace(Some(1));
        assert_eq!(result, "https://api.com/path/john?age=FUZZ29NUM&q=");

        let result = tokenized.replace(Some(2));
        assert_eq!(result, "https://api.com/path/john?age=29&q=FUZZSTR");

        let result = tokenized.replace(Some(3));
        assert_eq!(result, "https://api.com/path/john?age=29&q=");
    }
}
