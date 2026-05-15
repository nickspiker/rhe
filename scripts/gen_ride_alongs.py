#!/usr/bin/env python3
"""Generate ordered brief entries for free morphological ride-alongs.

Reads briefs.rs, en_freq.txt, and ordered_briefs claimed words.
Outputs Rust code snippets for ordered_briefs.rs and gen_briefs.rs.
"""

import re, sys
from pathlib import Path
from collections import defaultdict

ROOT = Path(__file__).resolve().parent.parent

# ── Scan constants ──────────────────────────────────────────────────
SCAN = {
    "R_THUMB": 57,
    "R_IDX": 36, "R_MID": 37, "R_RING": 38, "R_PINKY": 39,
    "L_IDX": 33, "L_MID": 32, "L_RING": 31, "L_PINKY": 30,
}

# Finger difficulty: easiest → hardest
# thumb < pinky < index < middle < ring
DIFFICULTY = {
    "R_THUMB": 0,
    "R_PINKY": 1, "L_PINKY": 1,
    "R_IDX": 2, "L_IDX": 2,
    "R_MID": 3, "L_MID": 3,
    "R_RING": 4, "L_RING": 4,
}

# False positives to reject — pairs that look like suffix relationships
# but aren't truly morphological.
FALSE_PAIRS = {
    ("as", "ass"), ("us", "using"), ("has", "ha"), ("off", "offer"),
    ("better", "bet"), ("bet", "better"), ("beautiful", "beauty"),
    ("beauty", "beautiful"), ("letter", "let"), ("let", "letter"),
    ("matter", "mat"), ("mat", "matter"), ("sounds", "so"),
    ("so", "sow"), ("so", "sos"), ("so", "sol"),
    ("or", "ore"), ("ore", "or"), ("an", "ans"),
    ("fire", "fir"), ("fir", "fire"),
    ("dinner", "din"), ("din", "dinner"),
    ("manner", "man"), ("man", "manner"),
    ("wonder", "wonderful"), ("wonderful", "wonder"),  # not suffix: wonder+ful but wonder is not a suffix-stripped form of wonderful
    ("apartment", "apart"),  # apart is not apartment-ment
    ("business", "busy"),  # not a suffix relationship
    ("certainly", "cert"),
    ("gone", "go"),  # gone is irregular, not go+ne
    ("done", "do"),  # irregular
    ("been", "be"),  # irregular
    ("came", "cam"),
    ("said", "sa"),
    ("left", "lef"),
    ("course", "cours"),
    ("moment", "mo"),
    ("front", "fro"),
    ("morning", "morn"),  # morn is archaic
    ("afternoon", "after"),  # not morphological
    ("honey", "hone"),  # different words
    ("money", "mone"),
    ("story", "stor"),
    ("funny", "fun"),  # not fun+ny morphologically (or is it?)
    ("country", "count"),  # not count+ry
    ("sorry", "sor"),
    ("hospital", "hospit"),
    ("ago", "ag"),
    ("pretty", "pret"),
    ("yesterday", "yes"),
    ("everybody", "every"),
    ("together", "to"),
    ("control", "con"),
    ("continue", "contin"),
    ("continues", "contin"),
    ("security", "secur"),
    ("police", "polic"),
    ("system", "syst"),
    ("general", "genera"),
    ("ahead", "ah"),
    ("appreciate", "appreci"),
    ("imagine", "imagin"),
    ("return", "ret"),
    ("secret", "secr"),
    ("responsible", "respons"),
    ("responsibility", "respons"),
    ("accident", "accid"),
    ("experience", "experi"),
    ("detective", "detect"),
    ("information", "inform"),
    ("congratulations", "congratul"),
    ("relationship", "relat"),
    ("absolutely", "absolut"),
    ("immediately", "immedi"),
    ("impossible", "impossibl"),
    ("christmas", "christ"),
    ("difficult", "diffic"),
    ("attention", "attent"),
    ("department", "depart"),
    ("evidence", "evid"),
    ("terrible", "terr"),
    ("situation", "situat"),
    ("welcome", "welcom"),
    # Not suffix relationships
    ("this", "thy"),        # th+is ≠ th+y
    ("ever", "eve"),        # eve+r is not -er suffix
    ("should", "shoulder"), # not should+er
    ("with", "wit"),        # not wit+h
    ("even", "eve"),        # not eve+n
    ("enough", "enou"),
    ("number", "numb"),     # not numb+er
    ("numb", "number"),
    ("remember", "rememb"),
    ("doctor", "doct"),
    ("door", "do"),
    ("bit", "bite"),        # not bit = bite - e
    ("might", "mite"),
    ("end", "en"),
    ("her", "here"),        # not here - e
    ("run", "rune"),
    ("rest", "reste"),
    ("sit", "site"),        # not site - e
    ("site", "sit"),
    ("mean", "mea"),
    ("plan", "plane"),      # not plane - e (different words)
    ("plane", "plan"),
    ("fine", "fin"),        # not fin+e
    ("fin", "fine"),
    ("cal", "call"),        # not call-l  (cal is not a real suffix strip)
    ("called", "cal"),
    ("call", "cal"),
    ("person", "pers"),
    ("bet", "bets"),        # bet is not a morphological base of better
    ("age", "aged"),        # if age is not briefed, skip
    ("star", "start"),      # not star+t
    ("start", "star"),
    ("water", "wate"),
    ("under", "und"),
    ("figure", "fig"),
    ("figure", "figur"),
    ("cover", "cove"),
    ("matter", "matt"),     # matt is a name
    ("car", "cared"), ("car", "caring"),  # cared/caring are from "care"
    ("me", "meal"), ("me", "ming"),  # not morphological
    ("alive", "al"),        # al is not alive-ive
    ("we", "wing"),         # not we+ing
    ("yes", "ye"),          # not y+es
    ("more", "moral"),      # not more+al
    ("thing", "th"),        # not morphological
    ("calling", "cal"),     # cal is a name
    ("called", "cal"),
    ("was", "wa"),          # not morphological
    ("fine", "final"),      # not fine+al
    ("bit", "bitter"),      # not bit+ter
    ("home", "homer"),      # homer is a proper noun
    ("scared", "scar"),     # different meaning
    ("relationship", "relate"),
    ("out", "outing"),      # not a useful suffix
    ("young", "you"),       # not you+ng
    ("different", "differ"),  # too distant
    ("interested", "inter"),
    ("interesting", "inter"),
    ("we", "weed"),         # not we+ed
    ("we", "weed"),
}

# ── Parse briefs.rs ──────────────────────────────────────────────────
def parse_briefs():
    """Returns dict: word -> (left, right)"""
    briefs = {}
    text = (ROOT / "src/layout/en/briefs.rs").read_text()
    for m in re.finditer(r'\(0b(\d+),\s*0b(\d+),\s*"(\w+)"\)', text):
        left = int(m.group(1), 2)
        right = int(m.group(2), 2)
        word = m.group(3)
        briefs[word] = (left, right)
    return briefs

# ── Parse frequencies ────────────────────────────────────────────────
def parse_freqs():
    freqs = {}
    for line in (ROOT / "data/en_freq.txt").open():
        parts = line.strip().split()
        if len(parts) == 2:
            freqs[parts[0]] = int(parts[1])
    return freqs

# ── Parse already-claimed ordered words ──────────────────────────────
def parse_ordered_claimed():
    """Returns set of words already in ordered_briefs.rs"""
    claimed = set()
    text = (ROOT / "src/layout/en/ordered_briefs.rs").read_text()
    for m in re.finditer(r'"(\w+)"', text):
        claimed.add(m.group(1))
    return claimed

# ── Enumerate fingers in a chord ────────────────────────────────────
def chord_fingers(left, right):
    """Returns list of (finger_name, scan_code) sorted by difficulty."""
    fingers = []
    if right & 0b10000: fingers.append(("R_THUMB", SCAN["R_THUMB"]))
    if right & 0b01000: fingers.append(("R_PINKY", SCAN["R_PINKY"]))
    if right & 0b00001: fingers.append(("R_IDX", SCAN["R_IDX"]))
    if right & 0b00010: fingers.append(("R_MID", SCAN["R_MID"]))
    if right & 0b00100: fingers.append(("R_RING", SCAN["R_RING"]))
    if left & 0b1000: fingers.append(("L_PINKY", SCAN["L_PINKY"]))
    if left & 0b0001: fingers.append(("L_IDX", SCAN["L_IDX"]))
    if left & 0b0010: fingers.append(("L_MID", SCAN["L_MID"]))
    if left & 0b0100: fingers.append(("L_RING", SCAN["L_RING"]))
    fingers.sort(key=lambda f: DIFFICULTY[f[0]])
    return fingers

# ── Find morphological relatives ────────────────────────────────────
SUFFIXES = ["s", "ed", "ing", "ly", "er", "tion", "al", "ment",
            "ness", "able", "ive", "ful", "ous", "ity"]

def find_relatives(word, freqs, min_freq=5000):
    """Find morphological relatives of a word that exist in freq table."""
    relatives = set()

    # Forward: word + suffix
    for suf in SUFFIXES:
        for candidate in _candidates_forward(word, suf):
            if candidate in freqs and freqs[candidate] >= min_freq:
                if (word, candidate) not in FALSE_PAIRS:
                    relatives.add(candidate)

    # Backward: strip suffix to find base
    for suf in SUFFIXES:
        for candidate in _candidates_backward(word, suf):
            if candidate in freqs and freqs[candidate] >= min_freq:
                if (word, candidate) not in FALSE_PAIRS:
                    relatives.add(candidate)

    return list(relatives)

def _candidates_forward(word, suf):
    """Generate candidate words by adding suffix to word."""
    yield word + suf
    # Drop final e before vowel-initial suffix
    if word.endswith("e") and suf[0] in "aeiou":
        yield word[:-1] + suf
    # Double final consonant (CVC pattern)
    if (len(word) >= 3 and word[-1] in "bdgklmnprst"
            and word[-2] in "aeiou" and word[-3] not in "aeiou"
            and suf[0] in "aeiou"):
        yield word + word[-1] + suf
    # y -> i before certain suffixes
    if word.endswith("y") and suf in ("ed", "er", "es", "ness"):
        yield word[:-1] + "i" + suf

def _candidates_backward(word, suf):
    """Generate candidate base words by stripping suffix."""
    if not word.endswith(suf) or len(word) <= len(suf) + 1:
        return
    base = word[:-len(suf)]
    yield base
    yield base + "e"  # restore dropped e
    # Un-double final consonant
    if len(base) >= 2 and base[-1] == base[-2]:
        yield base[:-1]
    # Restore y from i
    if base.endswith("i"):
        yield base[:-1] + "y"


def assign_fingers(all_words, fingers):
    """Assign words to fingers. Returns list of (word, [fingers])."""
    n_words = len(all_words)

    # Group fingers by difficulty tier
    tiers = defaultdict(list)
    for fname, fcode in fingers:
        tiers[DIFFICULTY[fname]].append((fname, fcode))
    sorted_tier_keys = sorted(tiers.keys())
    tier_list = [(t, tiers[t]) for t in sorted_tier_keys]
    n_tiers = len(tier_list)

    if n_words == 2:
        # Easy fingers → most common, hard finger(s) → rare
        has_ring = any(DIFFICULTY[f[0]] == 4 for f in fingers)
        threshold = 4 if has_ring else 3
        easy = [(f, c) for f, c in fingers if DIFFICULTY[f] < threshold]
        hard = [(f, c) for f, c in fingers if DIFFICULTY[f] >= threshold]
        if not hard:
            hard = [easy.pop()]
        return [(all_words[0], easy), (all_words[1], hard)]

    # N words, M tiers: give most-common word the easiest tier(s),
    # one tier per remaining word (least common → hardest).
    if n_words <= n_tiers:
        easy_tiers = n_tiers - (n_words - 1)
        assignments = []
        combined_easy = []
        for i in range(easy_tiers):
            combined_easy.extend(tier_list[i][1])
        assignments.append((all_words[0], combined_easy))
        for i in range(easy_tiers, n_tiers):
            word_idx = i - easy_tiers + 1
            assignments.append((all_words[word_idx], tier_list[i][1]))
        return assignments

    # More words than tiers — assign one word per tier, extras share
    # the hardest tier. Each word in the hardest tier needs its own
    # finger though — if we don't have enough fingers, drop words.
    assignments = []
    # Easy tiers: one word each
    for i in range(n_tiers - 1):
        assignments.append((all_words[i], tier_list[i][1]))
    # Hardest tier: remaining words each get one finger
    hardest_fingers = tier_list[-1][1]
    remaining_words = all_words[n_tiers - 1:]
    for j, w in enumerate(remaining_words):
        if j < len(hardest_fingers):
            assignments.append((w, [hardest_fingers[j]]))
    return assignments


def main():
    briefs = parse_briefs()
    freqs = parse_freqs()
    claimed = parse_ordered_claimed()
    brief_words = set(briefs.keys())

    # Find all ride-along opportunities
    raw_opportunities = []
    for word, (left, right) in briefs.items():
        if word in claimed:
            continue
        fingers = chord_fingers(left, right)
        if len(fingers) < 2:
            continue

        relatives = find_relatives(word, freqs)
        # Filter: not already claimed, not the brief word itself,
        # not already a separate auto-assigned brief
        relatives = [r for r in relatives
                     if r not in claimed and r != word and r not in brief_words]
        if not relatives:
            continue

        relatives.sort(key=lambda r: freqs.get(r, 0), reverse=True)

        # How many can we fit? Need 1+ fingers for brief word, rest for relatives.
        # Group by difficulty tier to count available tiers.
        tier_set = set()
        for fname, _ in fingers:
            tier_set.add(DIFFICULTY[fname])
        # Tiers available for relatives = total tiers - 1 (brief word gets easiest)
        # But within the hardest tier, each finger can hold one word.
        sorted_tiers = sorted(tier_set)
        if len(sorted_tiers) <= 1:
            # Only one difficulty tier — can still split if 2+ fingers
            max_relatives = len(fingers) - 1
        else:
            # Count: (n_tiers - 1) words get their own tier,
            # plus extra fingers in hardest tier can each hold a word
            hardest_tier = sorted_tiers[-1]
            hardest_count = sum(1 for f, _ in fingers if DIFFICULTY[f] == hardest_tier)
            # Available relative slots: (n_non_hardest_tiers - 1) + hardest_count
            # Actually: most common word gets tier 0 (and maybe tier 1...).
            # Each remaining tier holds one word. Hardest tier's fingers
            # can each hold a word if we have excess words.
            available_tiers = len(sorted_tiers) - 1  # tiers for relatives
            max_relatives = available_tiers
            if len(relatives) > available_tiers:
                # Pack extras into hardest tier
                max_relatives = available_tiers - 1 + hardest_count

        relatives = relatives[:max_relatives]
        if not relatives:
            continue

        total_rel_freq = sum(freqs.get(r, 0) for r in relatives)
        raw_opportunities.append((word, left, right, fingers, relatives, total_rel_freq))

    # Deduplicate: if the same relative word appears on multiple chords,
    # keep only the one with the highest-frequency brief word (most useful chord).
    word_best_chord = {}  # relative -> (brief_word, total_rel_freq, opportunity_index)
    for i, (word, left, right, fingers, relatives, total_freq) in enumerate(raw_opportunities):
        for rel in relatives:
            brief_freq = freqs.get(word, 0)
            existing = word_best_chord.get(rel)
            if existing is None or brief_freq > existing[0]:
                word_best_chord[rel] = (brief_freq, i)

    # Build final opportunities with deduplicated relatives
    opportunities = []
    for i, (word, left, right, fingers, relatives, _) in enumerate(raw_opportunities):
        kept = [r for r in relatives if word_best_chord.get(r, (0, -1))[1] == i]
        if not kept:
            continue
        total_freq = sum(freqs.get(r, 0) for r in kept)
        opportunities.append((word, left, right, fingers, kept, total_freq))

    # Sort by total relative frequency
    opportunities.sort(key=lambda o: o[5], reverse=True)

    # ── Generate Rust code ──────────────────────────────────────────
    print(f"    // ─── Free morphological ride-alongs ({len(opportunities)} sets) ────────")
    print(f"    // Each set reuses an existing auto-assigned brief chord.")
    print(f"    // The brief word keeps the easy finger(s); morphological")
    print(f"    // relatives ride on harder fingers at zero slot cost.")

    claimed_lines = []
    total_sets = 0
    total_relatives = 0

    for word, left, right, fingers, relatives, total_freq in opportunities:
        word_freq = freqs.get(word, 0)

        all_words = [(word, word_freq)] + [(r, freqs.get(r, 0)) for r in relatives]
        all_words.sort(key=lambda w: w[1], reverse=True)

        assignments = assign_fingers(all_words, fingers)

        left_bin = f"0b{left:04b}"
        right_bin = f"0b{right:05b}"
        word_list = " / ".join(f"{w}({f:,})" for w, f in all_words)
        print(f"    // {word_list}")

        for (w, wfreq), assigned_fingers in assignments:
            for fname, fcode in assigned_fingers:
                print(f"    ({left_bin}, {right_bin}, scan::{fname}, \"{w}\"),")
            claimed_lines.append(f"    (0b{right:05b}, 0b{left:04b}, \"{w}\"),")

        total_sets += 1
        total_relatives += len(relatives)

    print()
    print(f"    // {total_sets} sets, {total_relatives} ride-along words")
    print()
    print("=" * 72)
    print("// Add to ORDERED_CLAIMED in gen_briefs.rs:")
    print("    // Free morphological ride-alongs")
    for line in claimed_lines:
        print(line)

    print(f"\n// Total: {len(claimed_lines)} ORDERED_CLAIMED entries", file=sys.stderr)


if __name__ == "__main__":
    main()
