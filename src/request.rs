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

pub fn parse(url: &str, body: Option<&str>) -> Result<Request, url::ParseError> {
    let url = Url::parse(url)?;
    Ok(Request {
        origin: url.origin().ascii_serialization(),
        path: Tokenized::new(Some(url.path())),
        query: Tokenized::new(url.query()),
        body: Tokenized::new(body),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_candidate() {
        let request = parse(
            "https://api.com/shop/FUZZcustomersSTR/FUZZjohnSTR?age=FUZZ29NUM&q=FUZZSTR",
            Some(r#"{"one": FUZZ1NUM, "two": "2", "three": "FUZZaSTR"}"#),
        )
        .unwrap();

        let candidates = request.generate_candidates();
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
    fn test_parse() {
        let u = "https://api.com/shop/FUZZjohnSTR/FUZZSTR?age=FUZZ29NUM&q=FUZZSTR";
        let body = "{\"one\": FUZZ1NUM, \"two\": \"2\"}";
        let request = parse(u, Some(body)).unwrap();
        assert_eq!(request.path.size(), 2);
        assert_eq!(request.path.text, "/shop/FUZZjohnSTR/FUZZSTR");
        assert_eq!(request.query.size(), 2);
        assert_eq!(request.query.text, "age=FUZZ29NUM&q=FUZZSTR");
        assert_eq!(request.body.size(), 1);
        assert_eq!(request.body.text, body);
    }

    #[test]
    fn test_parse_empty() {
        let request = parse("https://example.com", None).unwrap();
        assert_eq!(request.path.size(), 0);
        assert_eq!(request.path.text, "/");
        assert_eq!(request.query.size(), 0);
        assert_eq!(request.query.text, "");
        assert_eq!(request.body.size(), 0);
        assert_eq!(request.body.text, "");
    }
}
