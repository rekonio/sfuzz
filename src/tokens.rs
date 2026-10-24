use std::fmt::{Display, Formatter};
use std::str::FromStr;
use regex::Regex;

struct FuzzToken {
    kind: FuzzKind,
    happy_value: String,
    start: usize,
    end: usize,
}

#[derive(Debug, PartialEq)]
enum FuzzKind {
    StringKind,
    NumeralKind,
}

const FUZZ_TOKEN_SIZE: usize = 7;


fn parse(text: &str) -> Vec<FuzzToken> {
    let mut tokens = Vec::new();
    let re = Regex::new(r"FUZZ(.*?)(STR|NUM)").unwrap();
    for c in re.captures_iter(text){
        let range = c.get(0).unwrap().range();
        let tok = FuzzToken {
            happy_value: c.get(1).unwrap().as_str().to_string(),
            kind: c.get(2).unwrap().as_str().parse().unwrap(),
            start: range.start,
            end: range.end,
        };
        tokens.push(tok);
    };
    tokens
}

fn replace(tokens: &[FuzzToken], text: &str, skip: Option<usize>) -> String {
    let mut text = String::from(text);
    let mut done: usize = 0;
    for (i, tok) in tokens.iter().enumerate() {
        if skip == Some(i) {
            continue;
        }
        let offset =  done *FUZZ_TOKEN_SIZE;
        let start = tok.start - offset;
        let end = tok.end - offset;
        text.replace_range(start..end, &tok.happy_value);
        done += 1;
    }
    text
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
    use crate::tokens::FuzzKind::{NumeralKind, StringKind};
    use super::*;

    #[test]
    fn test_parse() {
        let text = "https://api.com/path/FUZZjohnSTR?age=FUZZ29NUM&q=FUZZSTR";
        let tokens = parse(text);
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0].kind, StringKind);
        assert_eq!(tokens[0].happy_value, "john");
        assert_eq!(tokens[0].start, 21);
        assert_eq!(tokens[0].end, 32);
        assert_eq!(tokens[1].kind, NumeralKind);
        assert_eq!(tokens[1].happy_value, "29");
        assert_eq!(tokens[1].start, 37);
        assert_eq!(tokens[1].end, 46);
        assert_eq!(tokens[2].kind, StringKind);
        assert_eq!(tokens[2].happy_value, "");
        assert_eq!(tokens[2].start, 49);
        assert_eq!(tokens[2].end, 56);
    }

    #[test]
    fn test_replace() {
        let text = "https://api.com/path/FUZZjohnSTR?age=FUZZ29NUM&q=FUZZSTR";
        let tokens = parse(text);

        let result = replace(&tokens, text, None);
        assert_eq!(result, "https://api.com/path/john?age=29&q=");

        let result = replace(&tokens, text, Some(0));
        assert_eq!(result, "https://api.com/path/FUZZjohnSTR?age=29&q=");

        let result = replace(&tokens, text, Some(1));
        assert_eq!(result, "https://api.com/path/john?age=FUZZ29NUM&q=");

        let result = replace(&tokens, text, Some(2));
        assert_eq!(result, "https://api.com/path/john?age=29&q=FUZZSTR");

        let result = replace(&tokens, text, Some(3));
        assert_eq!(result, "https://api.com/path/john?age=29&q=");
    }
}