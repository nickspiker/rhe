# Rhe

From Greek *rheo* — to flow. A first-principles stenography engine.

10 keys. No finger movement. Chord syllables at the speed of thought.

## How it works

Your fingers rest on the home row. They never leave.

```
Left hand                         Right hand
pinky  ring  middle  index        index  middle  ring  pinky
  a      o      e      u            h      t      n      s

              [ctrl]  [space]
              left     right
              thumb    thumb
```

A chord is all fingers down, then all fingers up. The system reads:

- **Which fingers** on each hand (4 bits per hand, 16 states each)
- **Which hand pressed first** and **which released first** (4 modes)
- **Whether ctrl is held** (doubles the space)

That gives **2,048 distinct chords**. Each maps to a syllable.

## Typing a word

Hold space. Chord syllables. Release space.

```
space down
  chord → "su"    (fingers down, fingers up)
  chord → "per"   (fingers down, fingers up)
space up → outputs "super "
```

Space stays held for the entire word. Each finger release commits a syllable.
Releasing space emits the word followed by a space character.

## Without space

Chords without space held are **instant output** — no buffering:

- Single-chord whole words: "the", "and", "I", "you"
- Punctuation: . , ! ? : ; ' "
- Numbers: 0-9
- Navigation: enter, backspace, tab, esc

Another 2,048 slots.

## Undo

All 10 keys at once clears the word buffer.

## The four modes

Which hand goes down first and which comes up first creates four
distinct motor patterns — like four phases of a wave:

```
R down → L down → R up → L up     Mode 1
R down → L down → L up → R up     Mode 2
L down → R down → R up → L up     Mode 3
L down → R down → L up → R up     Mode 4
```

Not timing-dependent. You decide which hand leads and which hand
releases first. Two binary choices, four modes.

## The math

```
Right hand states:     16  (4 fingers, each on or off)
Left hand states:      16
Hand-order modes:       4
Ctrl modifier:          2
                     ────
Per-chord slots:    2,048

Coverage:
  828 syllables = 90% of English by usage
  1,505 syllables = 95%
  2,048 slots available

Without space:      2,048  (briefs, symbols, numbers, nav)
Total gestures:     4,096
```

## Why this layout

Every design decision was derived from statistical analysis of English
phoneme frequency (CMU Pronouncing Dictionary, 135K words) weighted by
real-world usage (OpenSubtitles corpus, 698M word tokens).

Key findings that shaped the design:

- 78% of English words by usage are single-syllable (one chord = one word)
- Average word is 1.29 syllables
- 90% of syllables are CV or CVC (one onset consonant, one coda)
- The same consonant map works for both hands because onset and coda
  consonant frequency distributions are different enough that collisions
  are rare (0.10% with optimal assignment)

The chord-to-syllable mapping is opaque (learned, not decomposed) but
structured: same right-hand shape generally means same onset sound, same
left-hand shape means same coda sound. Patterns emerge naturally.

## Status

Early development. Core engine compiles and passes tests. Not yet usable.

## Building

```
cargo build
cargo test
```

## License

TBD
