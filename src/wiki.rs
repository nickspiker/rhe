//! Random Wikipedia article extracts, for tutor practice text.
//!
//! Fetches plain-text summaries via the REST API, strips parentheticals
//! and non-English noise, splits into sentences, and caches to
//! `~/.cache/rhe/practice_wiki.txt` for about a week between refreshes.
//!
//! Falls back to the caller's bundled text if the network isn't there.

use std::path::PathBuf;
use std::time::Duration;

const USER_AGENT: &str = concat!(
    "rhe-tutor/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/nickspiker/rhe)"
);

const CACHE_TTL_SECS: u64 = 7 * 24 * 3600;
const ARTICLES_PER_FETCH: usize = 20;
const MIN_WORDS_PER_SENTENCE: usize = 6;
const MAX_WORDS_PER_SENTENCE: usize = 30;

/// Load practice sentences: cached copy if fresh, otherwise fetch fresh
/// and write the cache. Returns an empty Vec on network failure so the
/// caller can fall back to its own bundled text.
pub fn load_sentences() -> Vec<String> {
    let cache = cache_path();
    if let Some(sentences) = read_cache(&cache) {
        return sentences;
    }

    eprintln!("rhe: fetching Wikipedia practice text…");
    let extracts = match fetch_random_extracts(ARTICLES_PER_FETCH) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("rhe: wiki fetch failed ({}); using bundled text", e);
            return Vec::new();
        }
    };

    let mut sentences = Vec::new();
    let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
    for extract in &extracts {
        for s in split_sentences(&clean_extract(extract)) {
            if is_suitable(&s) && seen.insert(s.clone()) {
                sentences.push(s);
            }
        }
    }

    if sentences.is_empty() {
        return Vec::new();
    }

    if let Some(parent) = cache.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&cache, sentences.join("\n"));
    eprintln!("rhe: cached {} practice sentences", sentences.len());
    sentences
}

fn cache_path() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("rhe").join("practice_wiki.txt");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home)
            .join(".cache")
            .join("rhe")
            .join("practice_wiki.txt");
    }
    PathBuf::from(".rhe-cache").join("practice_wiki.txt")
}

fn read_cache(path: &PathBuf) -> Option<Vec<String>> {
    let meta = std::fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?;
    let age = modified.elapsed().ok()?;
    if age.as_secs() > CACHE_TTL_SECS {
        return None;
    }
    let text = std::fs::read_to_string(path).ok()?;
    let sentences: Vec<String> = text
        .lines()
        .map(str::to_string)
        .filter(|s| !s.is_empty())
        .collect();
    if sentences.is_empty() {
        None
    } else {
        Some(sentences)
    }
}

/// MediaWiki action API: one request returns N random article extracts.
/// This is properly random (generator=random with grnlimit) — the per-call
/// rest_v1/page/random/summary endpoint is CDN-cached and frequently
/// repeats the same article across consecutive calls.
fn fetch_random_extracts(count: usize) -> Result<Vec<String>, String> {
    let url = format!(
        "https://en.wikipedia.org/w/api.php?\
            action=query&format=json&\
            generator=random&grnlimit={}&grnnamespace=0&\
            prop=extracts&exintro=1&explaintext=1",
        count
    );
    let resp = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(20))
        .call()
        .map_err(|e| format!("{}", e))?;
    let body = resp.into_string().map_err(|e| format!("body: {}", e))?;
    let extracts = extract_all_json_strings(&body, "extract");
    if extracts.is_empty() {
        Err("no 'extract' fields in response".into())
    } else {
        Ok(extracts)
    }
}

/// Strip parenthetical/bracket content (pronunciations, dates, IPA),
/// drop non-ASCII-letter chars aside from basic punctuation, collapse
/// whitespace. Output is plain lowercase-friendly English prose.
fn clean_extract(text: &str) -> String {
    let without_brackets = strip_nested_brackets(text);
    let kept: String = without_brackets
        .chars()
        .map(|c| {
            if c.is_ascii_alphabetic()
                || c.is_ascii_digit()
                || matches!(c, ' ' | '\t' | '\n' | '.' | ',' | '\'' | '-' | '!' | '?' | ':' | ';')
            {
                c
            } else if !c.is_ascii() {
                ' '
            } else {
                ' '
            }
        })
        .collect();
    // Collapse runs of whitespace.
    kept.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn strip_nested_brackets(text: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in text.chars() {
        match c {
            '(' | '[' | '{' => depth += 1,
            ')' | ']' | '}' => {
                if depth > 0 {
                    depth -= 1;
                }
            }
            _ => {
                if depth == 0 {
                    out.push(c);
                }
            }
        }
    }
    out
}

fn split_sentences(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    for c in text.chars() {
        current.push(c);
        if matches!(c, '.' | '!' | '?') {
            let trimmed = current
                .trim()
                .trim_end_matches(|ch: char| !ch.is_alphanumeric())
                .to_string();
            if !trimmed.is_empty() {
                out.push(trimmed);
            }
            current.clear();
        }
    }
    let trimmed = current.trim();
    if !trimmed.is_empty() {
        out.push(trimmed.to_string());
    }
    out
}

fn is_suitable(sentence: &str) -> bool {
    let word_count = sentence.split_whitespace().count();
    if word_count < MIN_WORDS_PER_SENTENCE || word_count > MAX_WORDS_PER_SENTENCE {
        return false;
    }
    // Must be mostly letters (guards against garbled rows, tables, etc.)
    let alpha = sentence.chars().filter(|c| c.is_ascii_alphabetic()).count();
    let total = sentence.chars().count();
    alpha * 100 / total >= 70
}

/// Finds every `"<key>": "..."` string value in the JSON, in order.
/// Handles the escapes we see in Wikipedia output (\", \\, \n, \t, \/,
/// \uXXXX). Not a general JSON parser — just enough for flat string
/// fields nested inside the `pages` object.
fn extract_all_json_strings(json: &str, key: &str) -> Vec<String> {
    let needle = format!("\"{}\":", key);
    let mut results = Vec::new();
    let mut cursor = 0;
    while let Some(idx) = json[cursor..].find(&needle) {
        let start = cursor + idx + needle.len();
        let rest = json[start..].trim_start();
        let Some(remainder) = rest.strip_prefix('"') else {
            cursor = start + 1;
            continue;
        };
        let (value, consumed) = parse_json_string(remainder);
        if let Some(v) = value {
            results.push(v);
        }
        // Advance cursor past the end of this value, accounting for
        // whitespace skipped before the opening quote.
        let ws_skipped = rest.len() - remainder.len();
        cursor = start + ws_skipped + 1 + consumed + 1;
    }
    results
}

/// Given the contents of a JSON string starting AFTER the opening `"`,
/// return the unescaped value and the number of bytes consumed (up to
/// but not including the closing `"`).
fn parse_json_string(s: &str) -> (Option<String>, usize) {
    let mut chars = s.char_indices();
    let mut out = String::new();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return (Some(out), i),
            '\\' => match chars.next() {
                Some((_, '"')) => out.push('"'),
                Some((_, '\\')) => out.push('\\'),
                Some((_, '/')) => out.push('/'),
                Some((_, 'n')) => out.push('\n'),
                Some((_, 't')) => out.push(' '),
                Some((_, 'r')) => {}
                Some((_, 'u')) => {
                    let hex: String = chars.by_ref().take(4).map(|(_, c)| c).collect();
                    if hex.len() == 4 {
                        if let Ok(code) = u32::from_str_radix(&hex, 16) {
                            if let Some(ch) = char::from_u32(code) {
                                out.push(ch);
                            }
                        }
                    }
                }
                Some((_, ch)) => out.push(ch),
                None => return (None, i),
            },
            c => out.push(c),
        }
    }
    (None, s.len())
}
