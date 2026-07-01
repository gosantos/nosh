use std::collections::HashSet;

#[derive(Debug, Clone)]
pub struct FuzzyMatch {
    pub score: i64,
    pub indices: Vec<usize>,
}

pub fn fuzzy_match(query: &str, text: &str) -> Option<FuzzyMatch> {
    let query = query.trim();
    if query.is_empty() {
        return Some(FuzzyMatch {
            score: 0,
            indices: Vec::new(),
        });
    }

    let text_chars: Vec<char> = text.chars().collect();
    let query_chars: Vec<char> = query.chars().collect();
    let text_lower: Vec<char> = text.to_lowercase().chars().collect();
    let query_lower: Vec<char> = query.to_lowercase().chars().collect();

    let mut indices: Vec<usize> = Vec::with_capacity(query_chars.len());
    let mut text_idx = 0;
    let mut query_idx = 0;

    while text_idx < text_chars.len() && query_idx < query_chars.len() {
        if text_lower[text_idx] == query_lower[query_idx] {
            indices.push(text_idx);
            query_idx += 1;
        }
        text_idx += 1;
    }

    if query_idx < query_chars.len() {
        return None;
    }

    let score = score_match(&text_chars, &query_chars, &indices);
    Some(FuzzyMatch { score, indices })
}

fn score_match(text: &[char], query: &[char], indices: &[usize]) -> i64 {
    let mut score: i64 = 0;
    let mut prev: Option<usize> = None;
    let text_str: String = text.iter().collect();
    let query_str: String = query.iter().collect();

    for (i, &idx) in indices.iter().enumerate() {
        let ch = text[idx];

        // Base match score.
        score += 10;

        // Case-sensitive bonus when query char matches exactly.
        if i < query.len() && ch == query[i] {
            score += 5;
        }

        // First character match.
        if idx == 0 {
            score += 15;
        }

        // Word-boundary bonus.
        if idx == 0 || is_word_boundary(text, idx) {
            score += 15;
        }

        // Consecutive-match bonus.
        if let Some(p) = prev {
            if idx == p + 1 {
                score += 10;
            } else {
                // Slight gap penalty, but not too aggressive.
                score -= ((idx - p) as i64).min(10);
            }
        }

        prev = Some(idx);
    }

    // Exact substring bonus.
    if text_str.to_lowercase().contains(&query_str.to_lowercase()) {
        score += 20;
    }

    // Penalize long strings to prefer shorter matches.
    score -= (text.len() as i64 / 5).min(20);

    score
}

fn is_word_boundary(text: &[char], idx: usize) -> bool {
    let prev = text[idx - 1];
    let curr = text[idx];
    prev.is_whitespace()
        || prev == '-'
        || prev == '_'
        || prev == '/'
        || prev == '('
        || prev == '['
        || prev == '{'
        || (!prev.is_uppercase() && curr.is_uppercase())
}

/// Filters and scores a list of candidates. Empty query returns all with score 0.
pub fn filter<T: AsRef<str>>(query: &str, candidates: &[T]) -> Vec<(usize, FuzzyMatch)> {
    if query.trim().is_empty() {
        return candidates
            .iter()
            .enumerate()
            .map(|(i, _)| {
                (
                    i,
                    FuzzyMatch {
                        score: 0,
                        indices: Vec::new(),
                    },
                )
            })
            .collect();
    }

    let mut results: Vec<(usize, FuzzyMatch)> = candidates
        .iter()
        .enumerate()
        .filter_map(|(i, text)| fuzzy_match(query, text.as_ref()).map(|m| (i, m)))
        .collect();

    results.sort_by_key(|b| std::cmp::Reverse(b.1.score));
    results
}

/// Splits a string into styled segments for fuzzy-match highlighting.
pub struct HighlightedText {
    pub segments: Vec<(String, bool)>,
}

pub fn highlight(text: &str, indices: &[usize]) -> HighlightedText {
    let mut segments = Vec::new();
    if indices.is_empty() {
        segments.push((text.to_string(), false));
        return HighlightedText { segments };
    }

    let matched: HashSet<usize> = indices.iter().copied().collect();
    let chars: Vec<char> = text.chars().collect();
    let mut current = String::new();
    let mut current_matched: Option<bool> = None;

    for (idx, ch) in chars.iter().enumerate() {
        let is_match = matched.contains(&idx);
        if current_matched != Some(is_match) && !current.is_empty() {
            segments.push((current.clone(), current_matched.unwrap_or(false)));
            current.clear();
        }
        current.push(*ch);
        current_matched = Some(is_match);
    }

    if !current.is_empty() {
        segments.push((current, current_matched.unwrap_or(false)));
    }

    HighlightedText { segments }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_query_matches_everything() {
        let m = fuzzy_match("", "hello").unwrap();
        assert_eq!(m.score, 0);
        assert!(m.indices.is_empty());
    }

    #[test]
    fn simple_subsequence() {
        let m = fuzzy_match("abc", "aabbcc").unwrap();
        assert_eq!(m.indices, vec![0, 2, 4]);
        assert!(m.score > 0);
    }

    #[test]
    fn no_match() {
        assert!(fuzzy_match("xyz", "hello").is_none());
    }

    #[test]
    fn case_insensitive() {
        let m = fuzzy_match("hw", "Hello World").unwrap();
        assert_eq!(m.indices, vec![0, 6]);
    }

    #[test]
    fn filter_sorts_by_score() {
        let candidates = vec!["aabbcc", "abc", "xyz"];
        let results = filter("abc", &candidates);
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].0, 1); // exact-ish match wins
    }
}
