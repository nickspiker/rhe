# Rhe

<p align="center">
  <img src="logo.png" width="300" alt="Rhe">
</p>

From Greek *rheo* — to flow. A first-principles stenography engine.

10 keys. No finger movement. Chord syllables at the speed of thought.

## How it works

Your fingers rest on home row. They never leave. Rhe reads physical
key positions, not characters — it works with any OS keyboard layout.

```
Left hand                         Right hand
pinky  ring  middle  index        index  middle  ring  pinky

             [⌘]     [space]
             left      right
             thumb     thumb
```

A chord is: fingers down, fingers up. The system reads:

- **Which fingers** on each hand (4 bits per hand, 16 states each)
- **Which hand pressed first** and **which released first** (4 modes)
- **Whether ⌘ was tapped** during the chord (doubles the space)

That gives **2,048 distinct chords**. Each maps to a syllable.

## Consonants

Same map for both hands. Right hand = onset (beginning of syllable),
left hand = coda (end of syllable). ⌘ = the voiced or related partner
— same mouth position, same fingers.

```
Fingers     Sound         ⌘ Sound        Pair type
────────────────────────────────────────────────────
I           t (top)       d (do)          stop
M           n (no)        ŋ ng (ring)     nasal
I+M         s (so)        z (zoo)         fricative
R           r (run)       l (let)         liquid
I+R         m (me)        [spare]         nasal
M+R         k (can)       g (go)          stop
I+M+R       ð (the)       θ (think)       dental
P           w (we)        j y (you)       glide
I+P         h (he)        [spare]         glottal
M+P         b (but)       p (put)         stop
I+M+P       f (for)       v (very)        fricative
R+P         ʃ sh (she)    ʒ (measure)     postalveolar
I+R+P       dʒ (judge)    tʃ (church)     affricate
M+R+P       [spare]       [spare]
I+M+R+P     [spare]       [spare]
```

Assignments follow frequency × ease: the most common consonants land on
the easiest finger combos. Index (I) gets `t` — the most frequent consonant
in English. Adjacent finger pairs (I+M, M+R) are easier than gap pairs
(I+R, M+P). ⌘ pairs share the same base fingers so the motor pattern
transfers — `t` and `d` are the same finger, just add ⌘.

## Vowels

Four hand-ordering modes, each carrying a vowel. ⌘ = the stretched
or lengthened version of the same mouth position.

```
Voice    Order              Sound          ⌘ Sound
──────────────────────────────────────────────────────
zil      R↓ L↓ R↑ L↑       ʌ  uh (but)    ɑ  ah (father)
lun      L↓ R↓ L↑ R↑       ɪ  ih (sit)    iː ee (see)
ter      R↓ L↓ L↑ R↑       ɛ  eh (bed)    eɪ ay (say)
stel     L↓ R↓ R↑ L↑       æ  ah (cat)    aɪ eye (my)
```

Ordered by ease: same-exit patterns (zil, lun) are fastest because
the leading hand releases first. Right-lead (zil, ter) is faster than
left-lead (lun, stel) for right-hand dominant users. The most common
vowels land on the fastest modes.

Remaining vowels (oo, or, ow, er, oy, uh-book, ow-how) fill overflow
slots per onset+coda pair.

## Typing

**Single-chord words (no space):** Most common words in English are
single-syllable. Press and release — instant output. 78% of words by
usage are one chord.

```
chord → "the"     (right hand I+M+R, mode zil)
chord → "you"     (right hand R, no ⌘)
chord → "not"     (left hand I+M+R, no ⌘)
```

Top 60 words are assigned to single-hand chords by frequency. No
ordering needed — just press one hand and release.

**Multi-syllable words (hold space):** Space down at any point during
or before the chord marks it as a syllable. Chord syllables in sequence.
Release space to emit the word.

```
chord with space held → "su"     (buffered)
chord with space held → "per"    (buffered)
release space → outputs "super "
```

## Thumb controls

Left ⌘ is fully owned by rhe. Right ⌘ passes through for macOS
shortcuts (⌘+C, ⌘+Tab, etc.).

Thumb-only gestures (no fingers):

```
+space -space           enter
+⌘ -⌘                  backspace
(4 more two-thumb combos reserved for future use)
```

## The math

```
Right hand states:     16  (4 fingers, each on or off)
Left hand states:      16
Hand-order modes:       4
⌘ modifier:             2
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
- Consonant assignments are collision-minimized: the 4 single-key
  consonants (t, n, r, w) almost never cluster together in English
- ⌘ pairing follows linguistic voiced/unvoiced relationships, making
  the modifier semantically consistent rather than arbitrary

The chord-to-syllable mapping is opaque (each chord maps to a whole
syllable, not independently decoded) but structured: same right-hand
shape always means the same onset sound, same left-hand shape means
the same coda sound. Same mode always means the same vowel. Patterns
are consistent and learnable.

## Status

Early development. Core engine, menu bar app, IPA output mode, and
typing tutor are functional. Word output mode in progress.

```
rhe run       — menu bar app + full engine
rhe tutor     — interactive typing tutor
rhe map       — show consonant/vowel layout
rhe briefs    — show word brief assignments
rhe generate  — generate syllable table
rhe listen    — debug key events
```

## Building

```
cargo build --release
cargo test
```

## License

[Mozilla Public License 2.0](LICENSE)
