use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchDocument {
    pub id: String,
    pub title: String,
    pub body: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SearchResult {
    pub document_id: String,
    pub score: f32,
    pub matched_terms: Vec<String>,
}

pub fn lexical_search(documents: &[SearchDocument], query: &str) -> Vec<SearchResult> {
    let mut terms: Vec<String> = query
        .split_whitespace()
        .map(|term| term.to_owned())
        .filter(|term| !term.trim().is_empty())
        .collect();

    let chars: Vec<char> = query.chars().collect();
    for pair in chars.windows(2) {
        let bigram: String = pair.iter().collect();
        if !bigram.trim().is_empty() {
            terms.push(bigram);
        }
    }

    let mut results: Vec<SearchResult> = documents
        .iter()
        .filter_map(|document| {
            let matched: Vec<String> = terms
                .iter()
                .filter(|term| {
                    document.title.contains(term.as_str()) || document.body.contains(term.as_str())
                })
                .cloned()
                .collect();
            if matched.is_empty() {
                None
            } else {
                Some(SearchResult {
                    document_id: document.id.clone(),
                    score: matched.len() as f32,
                    matched_terms: matched,
                })
            }
        })
        .collect();

    results.sort_by(|left, right| right.score.total_cmp(&left.score));
    results
}
