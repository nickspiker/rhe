//! Runtime data loader.
//!
//! Three files are too large to ship inside the crate tarball (10 MiB crates.io limit), so they're resolved at runtime: `cmudict.dict` (CMU pronouncing dictionary), `en_freq.txt` (word frequency table), and `brown_corpus.txt.zst` (zstd-compressed Brown Corpus, used as the "Brown Corpus" tutor text source). Lookup order per file:
//!
//! 0. `$RHE_DATA_DIR/<file>` — explicit override
//! 1. `$XDG_CACHE_HOME/rhe/<file>` (or `~/.cache/rhe/<file>`)
//! 2. `./data/<file>` — convenience for running from a source checkout
//! 3. Download from a per-file URL into the cache, then read.
//!
//! Downloads happen once; subsequent runs hit the cache.
//!
//! `brown_corpus.txt.zst` was bundled with `rhe-0.1.0` on crates.io, so its download URL points at the docs.rs source view (`https://docs.rs/crate/rhe/0.1.0/source/assets/brown_corpus.txt.zst`) — that mirror is genuinely immutable: as long as version 0.1.0 exists on crates.io the bytes remain reachable, no GitHub-repo dependency. `cmudict.dict` and `en_freq.txt` were never small enough to publish, so they fall back to GitHub raw on `main`.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const GITHUB_RAW_BASE: &str = "https://raw.githubusercontent.com/nickspiker/rhe/main/data";
const BROWN_CORPUS_URL: &str = "https://docs.rs/crate/rhe/0.1.0/source/assets/brown_corpus.txt.zst";

pub fn load_cmudict() -> String {
    load_string("cmudict.dict")
}

pub fn load_word_freq() -> String {
    load_string("en_freq.txt")
}

/// Brown Corpus, zstd-compressed. Caller decompresses (decompressed size ~6 MB).
pub fn load_brown_corpus_zstd() -> Vec<u8> {
    load_bytes("brown_corpus.txt.zst")
}

fn load_string(filename: &str) -> String {
    let bytes = load_bytes(filename);
    String::from_utf8(bytes).unwrap_or_else(|e| panic!("rhe: {} not valid UTF-8: {}", filename, e))
}

fn load_bytes(filename: &str) -> Vec<u8> {
    for path in lookup_paths(filename) {
        if let Ok(contents) = fs::read(&path) {
            return contents;
        }
    }

    let cache = cache_path(filename);
    match download(filename, &cache) {
        Ok(()) => {
            fs::read(&cache).unwrap_or_else(|e| panic!("rhe: cached {:?} unreadable: {}", cache, e))
        }
        Err(e) => panic!(
            "rhe: could not load {}: {}\n\
             supply a local copy via $RHE_DATA_DIR or drop the file at {:?}",
            filename, e, cache
        ),
    }
}

fn lookup_paths(filename: &str) -> Vec<PathBuf> {
    let mut out = Vec::new();
    if let Ok(dir) = std::env::var("RHE_DATA_DIR") {
        out.push(PathBuf::from(dir).join(filename));
    }
    out.push(cache_path(filename));
    out.push(PathBuf::from("data").join(filename));
    out
}

fn cache_dir() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return PathBuf::from(xdg).join("rhe");
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        return PathBuf::from(home).join(".cache").join("rhe");
    }
    PathBuf::from(".rhe-cache")
}

fn cache_path(filename: &str) -> PathBuf {
    cache_dir().join(filename)
}

fn download_url(filename: &str) -> String {
    match filename {
        "brown_corpus.txt.zst" => BROWN_CORPUS_URL.to_string(),
        _ => format!("{}/{}", GITHUB_RAW_BASE, filename),
    }
}

fn download(filename: &str, dest: &Path) -> Result<(), String> {
    let url = download_url(filename);
    let response = ureq::get(&url)
        .call()
        .map_err(|e| format!("GET {}: {}", url, e))?;

    let mut reader = response.into_reader();
    let mut body = Vec::new();
    reader
        .read_to_end(&mut body)
        .map_err(|e| format!("read body: {}", e))?;

    if let Some(parent) = dest.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {:?}: {}", parent, e))?;
    }
    fs::write(dest, &body).map_err(|e| format!("write {:?}: {}", dest, e))?;
    Ok(())
}
