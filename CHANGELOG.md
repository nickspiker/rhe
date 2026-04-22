# Changelog

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
