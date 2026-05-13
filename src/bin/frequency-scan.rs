//! Sample text from one of several corpora, count Unicode codepoint frequencies, write a sorted CSV. Drives symbol-slot prioritization in `crate::layout::symbols` — the most frequent non-ASCII codepoints are the ones worth a single-stroke chord.
//!
//! Run with: `cargo run --release --bin frequency-scan`
//!
//! Sources (`--source`):
//! - `wikipedia` (default): random Wikipedia articles via the MediaWiki action API. Broad mixed-topic English text with occasional foreign-language names. Good baseline for "general written English".
//! - `arxiv`: arXiv abstract feed (math + physics categories). Math/physics-heavy text — the right corpus for ranking Greek consonants and math operators where the Wikipedia general-text scan is too sparse.
//!
//! Default: 100 batches × 10 articles ≈ 1000 abstracts. ~3–10 min depending on source latency. Tune with `--batches N` for a deeper sample. Output goes to `data/symbol_frequency.csv` by default; pass `--out PATH` to redirect.
//!
//! Failures are logged to stderr and skipped — partial samples still produce useful output.

use std::collections::HashMap;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const USER_AGENT: &str = concat!(
    "rhe-frequency-scan/",
    env!("CARGO_PKG_VERSION"),
    " (+https://github.com/nickspiker/rhe)"
);

/// Items per HTTP request, per source. Wikipedia's random generator caps unauthenticated callers at 10. arXiv tolerates much larger pages and aggressively rate-limits per-request, so a bigger page size means fewer round trips and avoids triggering the 429 wall on extended runs.
const WIKIPEDIA_ITEMS_PER_REQUEST: usize = 10;
const ARXIV_ITEMS_PER_REQUEST: usize = 100;

#[derive(Clone, Copy, Debug)]
enum Source {
    Wikipedia,
    Arxiv,
}

struct Args {
    batches: usize,
    out: PathBuf,
    include_ascii: bool,
    source: Source,
}

fn parse_args() -> Args {
    let mut batches = 100usize;
    let mut out = PathBuf::from("data/symbol_frequency.csv");
    let mut include_ascii = false;
    let mut source = Source::Wikipedia;
    let argv: Vec<String> = std::env::args().collect();
    let mut i = 1;
    while i < argv.len() {
        match argv[i].as_str() {
            "--batches" => {
                i += 1;
                batches = argv
                    .get(i)
                    .and_then(|s| s.parse().ok())
                    .expect("--batches expects a positive integer");
            }
            "--out" => {
                i += 1;
                out = PathBuf::from(argv.get(i).expect("--out expects a path"));
            }
            "--include-ascii" => {
                include_ascii = true;
            }
            "--source" => {
                i += 1;
                let raw = argv.get(i).expect("--source expects a name");
                source = match raw.as_str() {
                    "wikipedia" | "wiki" => Source::Wikipedia,
                    "arxiv" => Source::Arxiv,
                    other => {
                        eprintln!("unknown --source value: {} (expected wikipedia | arxiv)", other);
                        std::process::exit(1);
                    }
                };
            }
            "--help" | "-h" => {
                eprintln!(
                    "Usage: frequency-scan [--batches N] [--out PATH] [--include-ascii] [--source NAME]\n\
                     \n\
                     Streams text from a corpus, counts Unicode codepoints, writes sorted CSV.\n\
                     \n\
                     Sources:\n\
                       wikipedia (default)  random Wikipedia articles, mixed-topic English\n\
                       arxiv                arXiv math + physics abstracts (Greek/math-heavy)\n\
                     \n\
                     Defaults: 100 batches of 10 items → data/symbol_frequency.csv,\n\
                     non-ASCII codepoints only."
                );
                std::process::exit(0);
            }
            other => {
                eprintln!("unknown arg: {}", other);
                std::process::exit(1);
            }
        }
        i += 1;
    }
    Args {
        batches,
        out,
        include_ascii,
        source,
    }
}

fn main() {
    let args = parse_args();
    let start = Instant::now();

    let mut counts: HashMap<char, u64> = HashMap::new();
    let mut total_chars: u64 = 0;
    let mut total_articles: u64 = 0;
    let mut failed_batches: u64 = 0;

    for batch in 1..=args.batches {
        let result = match args.source {
            Source::Wikipedia => fetch_random_extracts(WIKIPEDIA_ITEMS_PER_REQUEST),
            Source::Arxiv => fetch_arxiv_abstracts(ARXIV_ITEMS_PER_REQUEST, batch - 1),
        };
        match result {
            Ok(extracts) => {
                for extract in &extracts {
                    for c in extract.chars() {
                        *counts.entry(c).or_insert(0) += 1;
                        total_chars += 1;
                    }
                    total_articles += 1;
                }
                eprintln!(
                    "batch {}/{}: +{} articles, total {} articles, {} chars",
                    batch,
                    args.batches,
                    extracts.len(),
                    total_articles,
                    total_chars,
                );
            }
            Err(e) => {
                failed_batches += 1;
                eprintln!("batch {}/{}: fetch failed: {}", batch, args.batches, e);
            }
        }
    }

    let mut sorted: Vec<(char, u64)> = counts
        .into_iter()
        .filter(|(c, _)| args.include_ascii || !c.is_ascii())
        .collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));

    if let Some(parent) = args.out.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    let mut f = match File::create(&args.out) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("failed to open {}: {}", args.out.display(), e);
            std::process::exit(1);
        }
    };
    writeln!(f, "codepoint,glyph,count,per_million,name_hint").unwrap();
    for (c, n) in &sorted {
        let glyph_field = if c.is_control() || *c == ',' || *c == '"' {
            String::new()
        } else {
            c.to_string()
        };
        let per_million = if total_chars > 0 {
            (*n as f64 * 1_000_000.0) / total_chars as f64
        } else {
            0.0
        };
        let name_hint = char_category_hint(*c);
        writeln!(
            f,
            "U+{:04X},{},{},{:.2},{}",
            *c as u32,
            glyph_field,
            n,
            per_million,
            name_hint,
        )
        .unwrap();
    }

    let elapsed = start.elapsed();
    eprintln!(
        "wrote {} ({} unique codepoints, {} articles, {} chars, {} failed batches) in {:.1}s",
        args.out.display(),
        sorted.len(),
        total_articles,
        total_chars,
        failed_batches,
        elapsed.as_secs_f64(),
    );
}

/// Coarse Unicode block hint to help eyeball the CSV without a separate Unicode database. Covers the ranges most likely to show up in mixed-language Wikipedia text — math, Greek, currency, dingbats, etc.
fn char_category_hint(c: char) -> &'static str {
    let cp = c as u32;
    match cp {
        0x0080..=0x00FF => "Latin-1 supplement",
        0x0100..=0x017F => "Latin Extended-A",
        0x0180..=0x024F => "Latin Extended-B",
        0x0250..=0x02AF => "IPA extensions",
        0x02B0..=0x02FF => "spacing modifiers",
        0x0300..=0x036F => "combining diacriticals",
        0x0370..=0x03FF => "Greek",
        0x0400..=0x04FF => "Cyrillic",
        0x0590..=0x05FF => "Hebrew",
        0x0600..=0x06FF => "Arabic",
        0x0900..=0x097F => "Devanagari",
        0x0E00..=0x0E7F => "Thai",
        0x1100..=0x11FF => "Hangul Jamo",
        0x1E00..=0x1EFF => "Latin Extended Additional",
        0x1F00..=0x1FFF => "Greek Extended",
        0x2000..=0x206F => "general punctuation",
        0x2070..=0x209F => "super/subscripts",
        0x20A0..=0x20CF => "currency",
        0x2100..=0x214F => "letterlike",
        0x2150..=0x218F => "number forms",
        0x2190..=0x21FF => "arrows",
        0x2200..=0x22FF => "math operators",
        0x2300..=0x23FF => "misc technical",
        0x2500..=0x257F => "box drawing",
        0x2580..=0x259F => "block elements",
        0x25A0..=0x25FF => "geometric shapes",
        0x2600..=0x26FF => "misc symbols",
        0x2700..=0x27BF => "dingbats",
        0x3000..=0x303F => "CJK punctuation",
        0x3040..=0x309F => "Hiragana",
        0x30A0..=0x30FF => "Katakana",
        0x3400..=0x4DBF => "CJK extension A",
        0x4E00..=0x9FFF => "CJK unified",
        0xAC00..=0xD7AF => "Hangul syllables",
        0xFB00..=0xFB4F => "alphabetic presentation",
        0xFE30..=0xFE4F => "CJK compatibility",
        0xFF00..=0xFFEF => "halfwidth/fullwidth",
        _ => "",
    }
}

fn fetch_random_extracts(count: usize) -> Result<Vec<String>, String> {
    // Full article text (no `exintro=1`): article INTROS are biased toward biographies and place descriptions, which over-weights diacritics and Cyrillic and almost never includes math symbols. Pulling the full body picks up formula sections, which is the signal we actually care about for symbol-slot prioritization.
    let url = format!(
        "https://en.wikipedia.org/w/api.php?\
            action=query&format=json&\
            generator=random&grnlimit={}&grnnamespace=0&\
            prop=extracts&explaintext=1",
        count
    );
    let resp = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(30))
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

/// arXiv API search across math + physics categories. `start` walks the result window so each batch returns different abstracts. The API returns Atom XML; we pull the `<summary>` blocks (each is one abstract) via tag-bracket extraction — good enough for plain prose, and arXiv abstracts don't HTML-escape the math symbols we care about (Greek letters, `± × ÷ ≤ ≥`, etc. appear as raw Unicode).
///
/// Search query: `cat:math.* OR cat:physics.*`. The API resolves wildcard prefixes against subject taxonomy roots, so this picks up math.AG (algebraic geometry), math.NT (number theory), physics.atom-ph, etc. — wide net for Greek-letter exposure.
///
/// Rate limit: arXiv's API guidelines ask for 3 seconds between requests. We sleep 3s before each call (skipped for batch 0) — slower than Wikipedia, but the arxiv corpus is the only way to get reliable Greek-consonant frequency data, so the wait is the price.
fn fetch_arxiv_abstracts(count: usize, batch_idx: usize) -> Result<Vec<String>, String> {
    if batch_idx > 0 {
        std::thread::sleep(Duration::from_secs(3));
    }
    let start = batch_idx * count;
    // https direct: arXiv redirects http→https and we eat the redirect cost otherwise.
    let url = format!(
        "https://export.arxiv.org/api/query?\
            search_query=cat:math.*+OR+cat:physics.*&\
            start={}&max_results={}&\
            sortBy=submittedDate&sortOrder=descending",
        start, count
    );
    let resp = ureq::get(&url)
        .set("User-Agent", USER_AGENT)
        .timeout(Duration::from_secs(60))
        .call()
        .map_err(|e| format!("{}", e))?;
    let body = resp.into_string().map_err(|e| format!("body: {}", e))?;
    let summaries = extract_xml_blocks(&body, "summary");
    if summaries.is_empty() {
        Err("no <summary> blocks in arXiv response".into())
    } else {
        Ok(summaries)
    }
}

/// Pull every `<tag>...</tag>` block out of an Atom/XML body. Doesn't unescape entities — fine for plain prose where the symbols of interest aren't `< > &`. Trims surrounding whitespace.
fn extract_xml_blocks(xml: &str, tag: &str) -> Vec<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(start) = xml[cursor..].find(&open) {
        let value_start = cursor + start + open.len();
        if let Some(end_off) = xml[value_start..].find(&close) {
            out.push(xml[value_start..value_start + end_off].trim().to_string());
            cursor = value_start + end_off + close.len();
        } else {
            break;
        }
    }
    out
}

/// Lifted from `tutor::wiki` so this binary stays self-contained — `src/bin/*` can't import from the main crate without going through a `[lib]` target. If the parser ever needs fixes, update both copies.
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
        let ws_skipped = rest.len() - remainder.len();
        cursor = start + ws_skipped + 1 + consumed + 1;
    }
    results
}

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
