//! Runtime data loader.
//!
//! The cmudict and word-frequency files are too large to ship inside the
//! crate tarball (10 MiB crates.io limit) so they're hosted on GitHub and
//! resolved at runtime. Lookup order per file:
//!
//!   1. `$RHE_DATA_DIR/<file>` — explicit override
//!   2. `$XDG_CACHE_HOME/rhe/<file>` (or `~/.cache/rhe/<file>`)
//!   3. `./data/<file>` — convenience for running from a source checkout
//!   4. Download from `https://raw.githubusercontent.com/nickspiker/rhe/main/data/<file>`
//!      into the cache directory, then read it.
//!
//! Downloads happen once; subsequent runs hit the cache.

use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};

const RAW_URL_BASE: &str = "https://raw.githubusercontent.com/nickspiker/rhe/main/data";

pub fn load_cmudict() -> String {
    load("cmudict.dict")
}

pub fn load_word_freq() -> String {
    load("en_freq.txt")
}

pub fn load_briefs() -> String {
    load("briefs.txt")
}

fn load(filename: &str) -> String {
    for path in lookup_paths(filename) {
        if let Ok(contents) = fs::read_to_string(&path) {
            return contents;
        }
    }

    let cache = cache_path(filename);
    // fetching data file from github
    match download(filename, &cache) {
        Ok(()) => fs::read_to_string(&cache)
            .unwrap_or_else(|e| panic!("rhe: cached {:?} unreadable: {}", cache, e)),
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

fn download(filename: &str, dest: &Path) -> Result<(), String> {
    let url = format!("{}/{}", RAW_URL_BASE, filename);
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
