# Changelog

## 0.1.4 — 2026-05-17

### Fixed
- **Tutor zoom step** — Ctrl+= / Ctrl+- / Ctrl+scroll were stepping by 10% per notch (multiplicative 1.1), so integer percentages like 101% were unreachable and every notch felt jerky. Ported photon's asymmetric `(33/32)^steps` / `(31/32)^(-steps)` math: per-step factors of 32/31 (in) and 32/33 (out), ~3% per step, and the product 1024/1023 means one in + one out returns nearly exactly to the originating ru. Mouse wheel normalizes `LineDelta` notches to 20px and divides by 20 to derive fractional steps, matching photon's path.

## 0.1.3 — 2026-05-16

### Added
- **Symbol mode** — new `+word +mod -word +chord` gesture enters a one-shot symbol-typing sub-session. Greek consonants (right hand) and Greek vowels + units (`° £ €`) + ASCII randos (`@ # $ ~ ; |` `` ` ``) on the left hand. Layout placed by glyph frequency × measured chord effort. Right-hand Greek consonant frequencies sourced from an arXiv math/physics-abstract scan; left-hand from a Wikipedia general-text scan.
- **Phoneme-mode mod-tap undo** — pressing+releasing mod while word is held backspaces one phoneme in the engine and clears the drill's errored state. Drill highlights the mod cell as the recovery target while errored mid-word; releasing everything still resets to step 0 as before.
- **Number-mode multi-finger extension** — multi-finger no-mod right-hand chords (effort 5-14) now emit Greek lowercase consonants matching symbol-mode placement. Left-hand multi-finger emits brackets and math operators (`[ ] { } < > √ ∞ ∂ ∫`).
- **Rheboard digit-position variant** — digits 4 and 5 also reachable via all-4-right and all-4-left chords (in addition to the inner-index keys), so layouts that omit the QWERTY G/H keys still have a complete number layout.
- **Effort ranking module** — `crate::layout::effort` exposes the 15-entry chord-shape ergonomic ranking as a single source of truth, consumed by symbol placement and available for future brief / phoneme layout work.
- **frequency-scan binary** — `cargo run --release --bin frequency-scan` samples a corpus and writes a sorted CSV of Unicode codepoint frequencies. `--source wikipedia` (default) pulls random Wikipedia articles; `--source arxiv` pulls math + physics abstracts (Greek/math-heavy). Used to data-rank symbol-mode slots.

### Changed
- **Event::Mod carries `activity_in_session`** — distinguishes pristine number-mode entry (activity=false → enter Number) from a mid-word mod-tap (activity=true → no-op or phoneme-undo). Fixes the silent-mode-switch bug where mod-tapping after a goof would flip the tutor's cell hints to digits while the drill was still in phoneme mode.
- **Tray menu: "Open Tutor" → "Tutor"** — name shortened. Clicking now always reloads Wikipedia content; previously a prior Test Text / Brown Corpus / dropped-file session would stick and the menu would no-op.
- **Symbol layout extracted to `crate::layout::symbols`** — was duplicated inline in `interpreter.rs` and `tutor/drill.rs`. Now a single array-driven module per language (`layout/en/symbols.rs`, `layout/mri/symbols.rs` stub) indexed by effort rank.

### Fixed
- Pristine-zero pristine-symbol-entry disambiguation — defers both `Mod` and `SpaceUp` after `-word` with thumb live, then either fires `SymbolMode` (chord-finger follow-up) or flushes both events (pristine-zero or anything else). Earlier passes only deferred `SpaceUp`.

## 0.0.2 — 2026-04-21

### Added
- **IOHIDManager driver** (macOS) — raw HID keyboard capture, one event
  per physical key state change, zero OS key repeat noise
- **evdev driver** (Linux) — pre-xkb scancode grab with uinput passthrough
  for non-rhe keys
- **Linux text output** — libxkbcommon reverse-map + uinput injection
  picks up the user's active xkb layout (Dvorak/Colemak/…) so
  emitted scancodes produce the right characters. `Ctrl+Shift+U <hex>
  Enter` fallback for chars outside the keymap (IPA, emoji). Env var
  `RHE_UNICODE_FALLBACK=off` suppresses the fallback when running
  against a terminal.
- **Linux `rhe run`** — full engine on Linux. Input-then-output ordering
  + self-grab filter prevents feedback loops. Caps Lock tap toggles
  rhe enabled/disabled (solo tap detection), Caps Lock + Esc quits.
  Esc alone passes through to the focused app.
- **Random practice text** — tutor fetches 20 random Wikipedia article
  extracts via the MediaWiki batch API (`generator=random&grnlimit=20`),
  strips parentheticals / brackets / non-ASCII, dedupes sentences by
  content, caches for a week at `~/.cache/rhe/practice_wiki.txt`.
  Falls back to bundled Alice in Wonderland if offline. Practice
  starts at a random sentence each launch (wraps around).
- **Interactive tutor** — step-by-step chord teaching with real-time key
  display, brief/phoneme mode switching, error recovery, word backtrack
- **Bench mode** (`rhe bench`) — measures chord speed per finger combo,
  continuous rounds, averaged results for mapping optimization
- **Roll system** — 496 word rolls ranked by `frequency * phonemes_saved`,
  right-only and two-hand slots ordered by measured effort
- **Suffix system** — 15 left-hand-only suffix rolls (-s, -ed, -ing, -ly,
  etc.) ordered by measured finger effort
- **Brief generator** (`gen_briefs`) — auto-generates roll assignments with
  pinned overrides, inflection filtering, proper noun exclusion,
  value-based ranking
- **Key passthrough** (macOS) — non-rhe keys re-injected via CGEvent so
  arrows, numbers, shortcuts all work while rhe is active
- **Backspace** — solo word tap deletes last emitted word (character-count
  aware for multi-byte IPA)

### Changed
- **Thumb swap** — spacebar = mod (right hand 5th bit), left command =
  word boundary. Mod is just another finger, no special treatment.
- **Phoneme mapping** — frequency-optimized from bench data. No more
  voiced/unvoiced pairing. T=index (most common → fastest chord).
- **State machine** — dual-mode firing: per-hand when word held (phonemes),
  all-zero when word not held (rolls). Rolling briefs work on 6KRO
  keyboards.
- **Purely event-driven** — no polling, no timers. Block on channel recv.
- **Tutor colour scheme** — word button active `#8000FF` purple filled
  block (hint dot `#400080` at half brightness), mod button active green
  `#00FF00` (hint dot `#007F00`), per-cell cyan→yellow gradient across
  the 10 finger positions, inner-index cells reserved for future digit
  mode.

## 0.0.1 — 2026-04-20

### Added
- Initial crates.io publish
- Core phoneme engine with CMU dict lookup
- Menu bar app (macOS)
- Basic tutor
- IPA fallback output
