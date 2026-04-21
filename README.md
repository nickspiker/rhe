# Rhe

<p align="center">
  <img src="https://raw.githubusercontent.com/nickspiker/rhe/main/logo.png" width="300" alt="Rhe">
</p>

From Greek *rheo* (ῥέω) — to flow.

A phonetic chord engine. You type sounds, not letters. No finger
movement, no spelling, no memorization of key positions. Just hold
the chord that makes the sound and let go.

## The idea

English has 39 sounds. This keyboard uses 9 keys to represent all of
them. Right hand handles consonants (5 bits), left hand handles
vowels (4 bits). You press the sound you want, release, and the
engine converts your phoneme sequence into the correct English word —
spelling, silent letters, and all.

Forget how to spell "subtle"? Doesn't matter. Chord S-AH-T-AH-L and
the engine figures out the rest.

## Layout

```
Left hand (vowels)             Right hand (consonants + mod)
pinky  ring  middle  index     index  middle  ring  pinky  [thumb]

         [word]
         left ⌘
```

- **Left hand**: 4 fingers = 15 vowel chords
- **Right hand**: 4 fingers + thumb (spacebar) = 24 consonant chords
- **Word key** (left ⌘): held during phoneme input, release commits the word

The thumb (spacebar) is the 5th bit for the right hand — it
distinguishes voiced from unvoiced consonants. Same finger position,
add thumb = voiced pair: T/D, S/Z, K/G, P/B, F/V, etc.

## Two modes

### Phoneme mode (word key held)

Hold the word key. Chord each sound in sequence. Release word key to
commit.

```
"cat" = [word + K] → [word + AE] → [word + T] → [release word]
```

The engine looks up the phoneme sequence in a 135,000-word
pronunciation dictionary and emits the correctly spelled word.

### Roll mode (word key not held)

Common words get shortcut "rolls" — press keys across both hands,
release everything. One gesture = one word. No word key needed.

496 words have rolls assigned. The top 50 get the fastest key
combinations (single finger, two-finger pairs). The rest map
phonetically: first consonant + first vowel of the word.

Rolls can be typed on 6-key-rollover keyboards by rolling hands
sequentially: left hand down → right hand down → left up → right up.
Keys never all hit zero until the roll is complete.

### Suffixes (left hand only, no word key)

15 suffix rolls for common endings:

```
Index           -s          Middle          -ing
Ring            -ed         Pinky           -'s
I+M             -ly         I+R             -er
M+R             -tion       M+P             -ment
R+P             -ness       I+M+R           -able
I+P             -ity        I+M+P           -ous
I+R+P           -ive        M+R+P           -al
I+M+R+P         -ful
```

Type "look" (roll) then middle finger alone = "looking". The suffix
backspaces the trailing space, appends itself, adds a new space.
Inflected forms ("looking", "wanted", "tries") are excluded from
rolls — use base word + suffix instead.

## Consonants

```
Fingers     Plain         +Thumb (voiced)
──────────────────────────────────────────
I           t             d
M           s             z
R           k             g
P           p             b
I+M         n             m
I+R         r             dh (the)
M+R         l             —
I+M+R       h             —
I+P         f             v
M+P         w             —
R+P         th (think)    —
I+M+P       sh            zh (measure)
I+R+P       ch (church)   jh (judge)
M+R+P       ng            —
I+M+R+P     y             —
```

15 unvoiced + 9 voiced = 24 consonants. 6 voiced slots spare.

## Vowels

```
Fingers     Sound           Example
─────────────────────────────────────
I           ʌ  (uh)         but, sofa
M           ɪ  (ih)         sit, kit
R           ɛ  (eh)         bed, dress
P           æ  (ae)         cat, trap
I+M         iː (ee)        see, fleece
I+R         ɑ  (ah)        father, lot
M+R         eɪ (ay)        say, face
I+M+R       ɝ  (er)        her, bird
I+P         aɪ (eye)       my, price
M+P         oʊ (oh)        go, goat
R+P         ɔ  (aw)        law, thought
I+M+P       uː (oo)       too, goose
I+R+P       aʊ (ow)       how, mouth
M+R+P       ʊ  (uh)        book, foot
I+M+R+P     ɔɪ (oy)        boy, choice
```

4 bits = 15 states = 15 vowels. Complete General American English
coverage.

## Speed

The alternating hand cadence enables continuous flow:

```
word ═══════════════════════════════════════
right    ══K══            ══T══
left           ══AE══
chord          ^K    ^AE        ^T
```

Each hand loads while the other fires. Phonemes overlap. Rolls
provide single-chord output for the 500 most common words.

```
Theoretical maximum:  ~325 WPM (perfect alternation)
Practical estimate:   ~195 WPM (60% efficiency)
With rolls:           ~300 WPM (60% of text is single-chord)
```

For comparison: average typist ~40 WPM, professional ~80 WPM,
court reporter steno ~225 WPM.

## Controls

```
Solo word tap (no fingers)     backspace last word
Word + mod tap (no fingers)    undo last phoneme (before commit)
```

## The math

```
Phonemes:    39 / 512 slots used (word held)
Rolls:       496 / 496 slots used (word not held)
Suffixes:    15 / 15 left-hand slots
Total:       550 gestures mapped

9 keys. 39 sounds. 500+ words instant. Complete English phoneme
coverage. Zero finger movement.
```

## Architecture

```
IOHIDManager / evdev     raw key events (1 per state change, no repeats)
        │
  State machine          word held → per-hand firing
        │                word not held → all-zero firing (rolls)
        │
   Interpreter           phoneme buffer → dictionary → word
        │                roll lookup → instant emit
        │                suffix → backspace + append
        │
 CGEvent / uinput        inject text into focused application
```

Purely event-driven. No polling, no timers, no key repeat. One
physical key change = one event = one state machine transition.

## Running

```
cargo run --release -- tutor     learn the chords
cargo run --release -- run       full engine (macOS menu bar)
cargo run --bin gen_briefs       regenerate roll assignments
cargo test                       verify everything
```

Requires Input Monitoring permission on macOS (keyboard seizure via
IOHIDManager). Run with `sudo` or add your terminal to System
Settings → Privacy & Security → Input Monitoring.

## Building

```
cargo build --release
cargo test
```

## License

[Mozilla Public License 2.0](LICENSE)
