use std::error::Error;
use std::fmt;
use std::str::FromStr;
use crate::tokens::Tokenized;
use url::Url;

#[derive(Debug, PartialEq)]
pub enum CandidateKind {
    Path,
    Query,
    Body,
}
pub struct RequestCandidate {
    kind: CandidateKind,
    method: String,
    url: Url,
    body: String,
}
pub struct Request {
    pub origin: String,
    pub method: String,
    pub path: Tokenized,
    pub query: Tokenized,
    pub body: Tokenized,
}

impl Request {
    pub fn generate_candidates(&self) -> Vec<RequestCandidate> {
        let base = Url::parse(&self.origin).unwrap();
        let happy_path = self.path.replace(None);
        let happy_query = self.query.replace(None);
        let happy_body = self.body.replace(None);

        let mut candidates = Vec::new();

        for i in 0..self.query.size() {
            let mut url = base.clone();
            url.set_path(&happy_path);
            url.set_query(Some(&self.query.replace(Some(i))));
            candidates.push(RequestCandidate {
                kind: CandidateKind::Query,
                method: "GET".to_string(),
                url,
                body: happy_body.to_string(),
            })
        }
        for i in 0..self.path.size() {
            let mut url = base.clone();
            url.set_path(&self.path.replace(Some(i)));
            url.set_query(Some(&happy_query));
            candidates.push(RequestCandidate {
                kind: CandidateKind::Path,
                method: "GET".to_string(),
                url,
                body: happy_body.to_string(),
            })
        }

        let mut happy_url = base.clone();
        happy_url.set_path(&happy_path);
        happy_url.set_query(Some(happy_query.as_str()));

        for i in 0..self.body.size() {
            candidates.push(RequestCandidate {
                kind: CandidateKind::Body,
                method: "GET".to_string(),
                url: happy_url.clone(),
                body: self.body.replace(Some(i)),
            })
        }
        candidates
    }
}


#[derive(Debug)]
struct RequestLine {
    method: String,
    url: Url,
    body: String,
}

impl RequestLine {
    pub fn to_request(&self) -> Request {
        Request {
            origin: self.url.origin().ascii_serialization(),
            method: self.method.to_string(),
            path: Tokenized::new(Some(self.url.path())),
            query: Tokenized::new(self.url.query()),
            body: Tokenized::new(Some(&self.body)),
        }
    }
}

impl FromStr for RequestLine {
    type Err = ParseError;
    fn from_str(line: &str) -> Result<Self, Self::Err> {
        let line = line.trim();
        if line.is_empty() {
            return Err(ParseError::Empty);
        }

        let (method, rest) = line
            .split_once(char::is_whitespace)
            .ok_or(ParseError::MissingUrl)?;

        let rest = rest.trim_start();
        let (url, body) = rest.split_once(char::is_whitespace).unwrap_or((rest, ""));

        let url = Url::parse(url).map_err(|source| ParseError::InvalidUrl {
            url: url.to_string(),
            source,
        })?;

        Ok(RequestLine {
            method: method.to_string(),
            url,
            body: body.trim().to_string(),
        })
    }
}

#[derive(Debug)]
pub enum ParseError {
    Empty,
    MissingUrl,
    InvalidUrl { url: String, source: url::ParseError },
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::Empty => write!(f, "empty line"),
            ParseError::MissingUrl => write!(f, "missing URL"),
            ParseError::InvalidUrl { url, source } => write!(f, "invalid URL {url:?}: {source}"),
        }
    }
}

impl Error for ParseError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ParseError::InvalidUrl { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_invalid_url() {
        let res = "GET not-a-url".parse::<RequestLine>();
        assert!(matches!(res, Err(ParseError::InvalidUrl { .. })));
    }
    #[test]
    fn generate_candidate() {
        let request = RequestLine::from_str(
            r#"GET https://api.com/shop/FUZZcustomersSTR/FUZZjohnSTR?age=FUZZ29NUM&q=FUZZSTR {"one": FUZZ1NUM, "two": "2", "three": "FUZZaSTR"}"#
        ).unwrap();

        let candidates = request.to_request().generate_candidates();
        assert_eq!(candidates.len(), 6);

        let path_candidates: Vec<_> = candidates
            .iter()
            .filter(|x| x.kind == CandidateKind::Path)
            .collect();
        assert_eq!(path_candidates.len(), 2);
        let item = path_candidates.get(0).unwrap();
        assert_eq!(
            "https://api.com/shop/FUZZcustomersSTR/john?age=29&q=",
            item.url.to_string()
        );
        assert_eq!(r#"{"one": 1, "two": "2", "three": "a"}"#, item.body);
        let item = path_candidates.get(1).unwrap();
        assert_eq!(
            "https://api.com/shop/customers/FUZZjohnSTR?age=29&q=",
            item.url.to_string()
        );
        assert_eq!(r#"{"one": 1, "two": "2", "three": "a"}"#, item.body);

        let query_candidates: Vec<_> = candidates
            .iter()
            .filter(|x| x.kind == CandidateKind::Query)
            .collect();
        assert_eq!(query_candidates.len(), 2);
        let item = query_candidates.get(0).unwrap();
        assert_eq!(
            "https://api.com/shop/customers/john?age=FUZZ29NUM&q=",
            item.url.to_string()
        );
        assert_eq!(r#"{"one": 1, "two": "2", "three": "a"}"#, item.body);
        let item = query_candidates.get(1).unwrap();
        assert_eq!(
            "https://api.com/shop/customers/john?age=29&q=FUZZSTR",
            item.url.to_string()
        );
        assert_eq!(r#"{"one": 1, "two": "2", "three": "a"}"#, item.body);

        let body_candidates: Vec<_> = candidates
            .iter()
            .filter(|x| x.kind == CandidateKind::Body)
            .collect();
        assert_eq!(body_candidates.len(), 2);
        let item = body_candidates.get(0).unwrap();
        assert_eq!(
            "https://api.com/shop/customers/john?age=29&q=",
            item.url.to_string()
        );
        assert_eq!(r#"{"one": FUZZ1NUM, "two": "2", "three": "a"}"#, item.body);
        let item = body_candidates.get(1).unwrap();
        assert_eq!(
            "https://api.com/shop/customers/john?age=29&q=",
            item.url.to_string()
        );
        assert_eq!(r#"{"one": 1, "two": "2", "three": "FUZZaSTR"}"#, item.body);
    }

    #[test]
    fn parse_request() {
        let body = r#"{"one": FUZZ1NUM, "two": "2"}"#;
        let u = format!("POST https://api.com/shop/FUZZjohnSTR/FUZZSTR?age=FUZZ29NUM&q=FUZZSTR {}", body);
        let request = RequestLine::from_str(&u).unwrap().to_request();
        assert_eq!(request.method, "POST");
        assert_eq!(request.path.size(), 2);
        assert_eq!(request.path.text, "/shop/FUZZjohnSTR/FUZZSTR");
        assert_eq!(request.query.size(), 2);
        assert_eq!(request.query.text, "age=FUZZ29NUM&q=FUZZSTR");
        assert_eq!(request.body.size(), 1);
        assert_eq!(request.body.text, body);
    }

    #[test]
    fn parse_empty() {
        let request = RequestLine::from_str("PUT https://example.com").unwrap().to_request();
        assert_eq!(request.method, "PUT");
        assert_eq!(request.path.size(), 0);
        assert_eq!(request.path.text, "/");
        assert_eq!(request.query.size(), 0);
        assert_eq!(request.query.text, "");
        assert_eq!(request.body.size(), 0);
        assert_eq!(request.body.text, "");
    }
}
