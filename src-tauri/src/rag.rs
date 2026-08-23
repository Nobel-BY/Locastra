use crate::models::KnowledgeSnippet;
use std::collections::{HashMap, HashSet};

pub const EMBEDDING_DIMENSIONS: usize = 384;

#[derive(Debug, Clone)]
pub struct IndexedChunk {
    pub id: String,
    pub document_id: String,
    pub document_name: String,
    pub source_path: String,
    pub chunk_index: usize,
    pub page: Option<u32>,
    pub content: String,
    pub terms: Vec<String>,
    pub embedding: Vec<f32>,
}

pub fn tokenize(text: &str) -> Vec<String> {
    let lower = text.to_lowercase();
    let chars: Vec<char> = lower.chars().collect();
    let mut output = Vec::new();
    let mut latin = String::new();
    for (index, ch) in chars.iter().enumerate() {
        if ch.is_ascii_alphanumeric() || *ch == '_' {
            latin.push(*ch);
        } else {
            if latin.len() > 1 {
                output.push(std::mem::take(&mut latin));
            } else {
                latin.clear();
            }
            if !ch.is_whitespace() && !ch.is_ascii_punctuation() {
                output.push(ch.to_string());
                if let Some(next) = chars.get(index + 1) {
                    if !next.is_whitespace() && !next.is_ascii_punctuation() {
                        output.push(format!("{ch}{next}"));
                    }
                }
            }
        }
    }
    if latin.len() > 1 {
        output.push(latin);
    }
    output
}

fn hash64(value: &str) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn embed(text: &str) -> Vec<f32> {
    let mut vector = vec![0.0f32; EMBEDDING_DIMENSIONS];
    for term in tokenize(text) {
        let hash = hash64(&term);
        let index = (hash as usize) % EMBEDDING_DIMENSIONS;
        let sign = if (hash >> 63) == 0 { 1.0 } else { -1.0 };
        vector[index] += sign;
    }
    let norm = vector.iter().map(|value| value * value).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in &mut vector {
            *value /= norm;
        }
    }
    vector
}

pub fn chunk_text(text: &str, target_chars: usize, overlap_chars: usize) -> Vec<String> {
    let chars: Vec<char> = text.chars().collect();
    if chars.is_empty() {
        return Vec::new();
    }
    let target = target_chars.max(300);
    let overlap = overlap_chars.min(target / 2);
    let mut chunks = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let hard_end = (start + target).min(chars.len());
        let mut end = hard_end;
        if hard_end < chars.len() {
            let search_start = start + target * 2 / 3;
            if let Some(boundary) = (search_start..hard_end)
                .rev()
                .find(|index| matches!(chars[*index], '\n' | '。' | '！' | '？' | '.' | '!' | '?'))
            {
                end = boundary + 1;
            }
        }
        let content: String = chars[start..end]
            .iter()
            .collect::<String>()
            .trim()
            .to_string();
        if !content.is_empty() {
            chunks.push(content);
        }
        if end >= chars.len() {
            break;
        }
        start = end.saturating_sub(overlap);
    }
    chunks
}

fn cosine(left: &[f32], right: &[f32]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| (*a as f64) * (*b as f64))
        .sum()
}

pub fn search(chunks: &[IndexedChunk], query: &str, limit: usize) -> Vec<KnowledgeSnippet> {
    if chunks.is_empty() || query.trim().is_empty() {
        return Vec::new();
    }
    let query_terms = tokenize(query);
    let query_set: HashSet<&str> = query_terms.iter().map(String::as_str).collect();
    let query_embedding = embed(query);
    let average_length = chunks.iter().map(|chunk| chunk.terms.len()).sum::<usize>() as f64
        / chunks.len().max(1) as f64;
    let mut document_frequency: HashMap<&str, usize> = HashMap::new();
    for chunk in chunks {
        let unique: HashSet<&str> = chunk.terms.iter().map(String::as_str).collect();
        for term in query_set.intersection(&unique) {
            *document_frequency.entry(term).or_default() += 1;
        }
    }
    let mut ranked = chunks
        .iter()
        .map(|chunk| {
            let frequencies =
                chunk
                    .terms
                    .iter()
                    .fold(HashMap::<&str, usize>::new(), |mut map, term| {
                        *map.entry(term.as_str()).or_default() += 1;
                        map
                    });
            let mut bm25 = 0.0;
            for term in &query_terms {
                let frequency = *frequencies.get(term.as_str()).unwrap_or(&0) as f64;
                if frequency == 0.0 {
                    continue;
                }
                let df = *document_frequency.get(term.as_str()).unwrap_or(&0) as f64;
                let idf = (((chunks.len() as f64 - df + 0.5) / (df + 0.5)) + 1.0).ln();
                let length = chunk.terms.len() as f64;
                bm25 += idf * (frequency * 2.2)
                    / (frequency + 1.2 * (1.0 - 0.75 + 0.75 * length / average_length.max(1.0)));
            }
            let vector_score = cosine(&query_embedding, &chunk.embedding).max(0.0);
            let phrase_bonus = if chunk.content.to_lowercase().contains(&query.to_lowercase()) {
                0.18
            } else {
                0.0
            };
            let bm25_normalized = bm25 / (bm25 + 4.0);
            let score = vector_score * 0.38 + bm25_normalized * 0.52 + phrase_bonus;
            KnowledgeSnippet {
                chunk_id: chunk.id.clone(),
                document_id: chunk.document_id.clone(),
                document_name: chunk.document_name.clone(),
                source_path: chunk.source_path.clone(),
                chunk_index: chunk.chunk_index as u32,
                page: chunk.page,
                content: chunk.content.clone(),
                score,
                vector_score,
                bm25_score: bm25_normalized,
            }
        })
        .collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        right
            .score
            .partial_cmp(&left.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    ranked.truncate(limit.max(1));
    ranked
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chinese_hybrid_search_prefers_relevant_chunk() {
        let make = |id: &str, text: &str| IndexedChunk {
            id: id.into(),
            document_id: "doc".into(),
            document_name: "测试.md".into(),
            source_path: "test.md".into(),
            chunk_index: 0,
            page: None,
            content: text.into(),
            terms: tokenize(text),
            embedding: embed(text),
        };
        let chunks = vec![
            make("a", "北京今天下雨，记得带伞。"),
            make("b", "本地大模型可以使用 CUDA 加速推理。"),
        ];
        assert_eq!(search(&chunks, "CUDA 推理如何加速", 1)[0].chunk_id, "b");
    }

    #[test]
    fn chunks_keep_overlap_and_content() {
        let chunks = chunk_text(&"甲".repeat(1800), 900, 100);
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|chunk| !chunk.is_empty()));
    }
}
