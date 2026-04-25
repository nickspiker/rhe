//! Random Wikipedia article extracts, for tutor practice text.
//!
//! Fetches plain-text summaries via the MediaWiki action API on
//! demand — no cache. Two entry points:
//!
//! - `load_sentences()` — blocking fetch of one article's sentences.
//!   Returns an empty Vec on network failure so the caller can fall
//!   back to bundled text.
//! - `SentenceStream` — double-buffered: fetches one article up-front
//!   (the initial batch blocks), then pre-fetches the next in a
//!   background thread. The tutor pulls the next batch on a
//!   non-blocking poll when it finishes the current one, and that
//!   pull kicks off the following prefetch. Steady state is one
//!   always-prefetched article sitting ready.

use std::sync::mpsc;
use std::time::Duration;

const USER_AGENT: &str = concat!(
    "rhe-tutor/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/nickspiker/rhe)"
);

const ARTICLES_PER_FETCH: usize = 1;
const MIN_WORDS_PER_SENTENCE: usize = 6;
const MAX_WORDS_PER_SENTENCE: usize = 30;

pub fn load_sentences() -> Vec<String> {
    fetch_batch()
}

/// Perform one fetch and turn the result into cleaned sentences.
/// Separate from the public entry so `SentenceStream` can reuse it
/// from a worker thread.
fn fetch_batch() -> Vec<String> {
    let extracts = match fetch_random_extracts(ARTICLES_PER_FETCH) {
        Ok(e) => e,
        Err(e) => {
            // wiki fetch failed, fall back to bundled text
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

    sentences
}

/// Double-buffered article stream. `initial()` blocks until the first
/// batch is ready; at the same time a background thread starts
/// prefetching the next. `try_next()` is non-blocking: if the
/// prefetch is done, it returns the ready batch and kicks off the
/// next prefetch; otherwise `None`.
pub struct SentenceStream {
    rx: mpsc::Receiver<Vec<String>>,
    tx: mpsc::Sender<Vec<String>>,
}

impl SentenceStream {
    /// Spawn the first fetch immediately. Call `initial()` to block
    /// for it.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        spawn_fetch(tx.clone());
        Self { rx, tx }
    }

    /// Block until the first batch arrives, then kick off the next
    /// prefetch so it's ready by the time the tutor needs it.
    pub fn initial(&self) -> Vec<String> {
        let first = self.rx.recv().unwrap_or_default();
        spawn_fetch(self.tx.clone());
        first
    }

    /// Non-blocking poll for the next prefetched batch. If one is
    /// ready, returns it and immediately kicks off another fetch so
    /// there's always exactly one prefetch in flight.
    pub fn try_next(&self) -> Option<Vec<String>> {
        let batch = self.rx.try_recv().ok()?;
        spawn_fetch(self.tx.clone());
        Some(batch)
    }
}

fn spawn_fetch(tx: mpsc::Sender<Vec<String>>) {
    std::thread::spawn(move || {
        let _ = tx.send(fetch_batch());
    });
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
