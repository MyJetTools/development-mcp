use ahash::AHashMap;



/// Lexical (BM25) half of the search.
///
/// Worth having next to the dense index because this corpus is full of exact
/// identifiers - `FlUrl`, `ArcSwap`, `PartitionKey`, `with_retries`. An
/// embedding turns a precise name into a cloud of meaning; BM25 keeps it a
/// name. It also gets "not in the corpus" right for free: a word that appears
/// nowhere scores zero rather than 0.81.
pub struct Bm25Index {
    /// Term frequencies per chunk, parallel to the chunk list.
    doc_terms: Vec<AHashMap<String, u32>>,
    doc_len: Vec<f32>,
    avg_len: f32,
    /// In how many chunks each term appears.
    df: AHashMap<String, u32>,
    docs_amount: usize,
}

/// Splits on anything that is not alphanumeric or `_`, lowercases, and - for
/// compound identifiers - additionally emits the parts.
///
/// `with_retries` yields `with_retries`, `with`, `retries`; `PartitionKey`
/// yields `partitionkey`, `partition`, `key`. So a query can hit either the
/// exact identifier or the words inside it. Unicode-aware, so Russian queries
/// tokenize normally too.
pub fn tokenize(text: &str) -> Vec<String> {
    let mut result = Vec::new();
    let mut current = String::new();

    for ch in text.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            current.push(ch);
            continue;
        }

        if !current.is_empty() {
            push_token(&mut result, &current);
            current.clear();
        }
    }

    if !current.is_empty() {
        push_token(&mut result, &current);
    }

    result
}

fn push_token(result: &mut Vec<String>, raw: &str) {
    let whole = raw.to_lowercase();
    result.push(whole.clone());

    let mut parts: Vec<String> = Vec::new();

    for segment in raw.split('_') {
        if segment.is_empty() {
            continue;
        }

        let mut piece = String::new();

        for ch in segment.chars() {
            if ch.is_uppercase() && !piece.is_empty() {
                parts.push(piece.to_lowercase());
                piece.clear();
            }

            piece.push(ch);
        }

        if !piece.is_empty() {
            parts.push(piece.to_lowercase());
        }
    }

    if parts.len() < 2 {
        return;
    }

    for part in parts {
        if part != whole {
            result.push(part);
        }
    }
}

impl Bm25Index {
    pub fn build(texts: &[String]) -> Self {
        let mut doc_terms = Vec::with_capacity(texts.len());
        let mut doc_len = Vec::with_capacity(texts.len());
        let mut df: AHashMap<String, u32> = AHashMap::new();
        let mut total_len = 0.0;

        for text in texts {
            let tokens = tokenize(text);

            total_len += tokens.len() as f32;
            doc_len.push(tokens.len() as f32);

            let mut terms: AHashMap<String, u32> = AHashMap::new();

            for token in tokens {
                *terms.entry(token).or_insert(0) += 1;
            }

            for term in terms.keys() {
                *df.entry(term.clone()).or_insert(0) += 1;
            }

            doc_terms.push(terms);
        }

        let docs_amount = texts.len();

        let avg_len = if docs_amount == 0 {
            0.0
        } else {
            total_len / docs_amount as f32
        };

        Self {
            doc_terms,
            doc_len,
            avg_len,
            df,
            docs_amount,
        }
    }

    /// Scores every chunk that shares at least one term with the query and
    /// returns them best first. Chunks with no shared term are left out
    /// entirely rather than scored as a weak match.
    pub fn search(&self, query: &str, k1: f32, b: f32) -> Vec<(usize, f32)> {
        if self.docs_amount == 0 || self.avg_len == 0.0 {
            return Vec::new();
        }

        let mut query_terms: AHashMap<String, u32> = AHashMap::new();

        for token in tokenize(query) {
            *query_terms.entry(token).or_insert(0) += 1;
        }

        let mut scores = vec![0.0f32; self.docs_amount];

        for term in query_terms.keys() {
            let Some(df) = self.df.get(term) else {
                continue;
            };

            let df = *df as f32;
            let n = self.docs_amount as f32;
            let idf = (1.0 + (n - df + 0.5) / (df + 0.5)).ln();

            for (index, terms) in self.doc_terms.iter().enumerate() {
                let Some(tf) = terms.get(term) else {
                    continue;
                };

                let tf = *tf as f32;
                let norm = 1.0 - b + b * self.doc_len[index] / self.avg_len;

                scores[index] += idf * (tf * (k1 + 1.0)) / (tf + k1 * norm);
            }
        }

        let mut result: Vec<(usize, f32)> = scores
            .into_iter()
            .enumerate()
            .filter(|(_, score)| *score > 0.0)
            .collect();

        result.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compound_identifiers_are_split_and_kept() {
        let tokens = tokenize("call writer.with_retries(3) on PartitionKey");

        for expected in [
            "with_retries",
            "retries",
            "partitionkey",
            "partition",
            "key",
        ] {
            assert!(tokens.contains(&expected.to_string()), "missing {}", expected);
        }
    }

    #[test]
    fn a_word_absent_from_the_corpus_scores_nothing() {
        let docs = vec![
            "FlUrl is the HTTP client used everywhere".to_string(),
            "ArcSwap holds read mostly state".to_string(),
        ];

        let index = Bm25Index::build(&docs);

        assert!(index.search("kafka consumer group", 1.2, 0.75).is_empty());
        assert!(!index.search("http client", 1.2, 0.75).is_empty());
    }
}
