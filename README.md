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

496 words have rolls assigned, ranked by `frequency × phonemes_saved`.
Right-hand-only slots (31) go to 2+ phoneme words. Two-hand slots
(465) go to 3+ phoneme words. Effort ordering measured on real hands
via `rhe bench`.

Rolls can be typed on 6-key-rollover keyboards by rolling hands
sequentially: left hand down → right hand down → left up → right up.
Keys never all hit zero until the roll is complete.

### Suffixes (left hand only, no word key)

15 suffix rolls, ordered by measured finger speed:

```
Index           -s          Ring            -ed
Pinky           -ing        Middle          -ly
All four        -'s         Middle+Ring     -er
Index+Middle    -tion       I+M+R           -al
Index+Pinky     -ment       Index+Ring      -ness
Ring+Pinky      -able       M+R+P           -ive
Middle+Pinky    -ful        I+R+P           -ous
I+M+P           -ity
```

Type "look" (roll) then index alone = "looks". The suffix
backspaces the trailing space, appends itself, adds a new space.
Inflected forms ("looking", "wanted", "tries") are excluded from
rolls — use base word + suffix instead.

## Consonants

Mapped by frequency × measured chord effort. No voicing pairs —
pure speed optimization. The most common consonant (T) gets the
fastest chord (index). Assignments measured on real hands via
`rhe bench`.

```
Fingers     No thumb        +Thumb
──────────────────────────────────────────
I           t               n
R           s               v
P           d               ng
M           r               g
All four    m               sh
M+R         l               th (think)
I+M         k               jh (judge)
I+M+R       dh (the)        ch (church)
I+P         w               zh (measure)
I+R         z               — spare
R+P         y               — spare
M+R+P       h               — spare
M+P         f               — spare
I+R+P       b               — spare
I+M+P       p               — spare
```

15 without thumb + 9 with thumb = 24 consonants. 6 thumb slots spare.

## Vowels

Mapped by frequency onto left hand, same effort ranking:

```
Fingers     Sound           Example
─────────────────────────────────────
I           ʌ  (uh)         but, sofa
R           ɪ  (ih)         sit, kit
P           iː (ee)        see, fleece
M           ɛ  (eh)         bed, dress
All four    uː (oo)        too, goose
M+R         aɪ (eye)       my, price
I+M         æ  (ae)         cat, trap
I+M+R       ɑ  (ah)        father, lot
I+P         ɝ  (er)        her, bird
I+R         oʊ (oh)        go, goat
R+P         eɪ (ay)        say, face
M+R+P       ɔ  (aw)        law, thought
M+P         aʊ (ow)        how, mouth
I+R+P       ʊ  (uh)        book, foot
I+M+P       ɔɪ (oy)        boy, choice
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
IOHIDManager (macOS)     raw key events (1 per state change, no repeats)
evdev grab (Linux)       — pre-xkb scancodes, layout-independent
        │
  State machine          word held → per-hand firing
        │                word not held → all-zero firing (rolls)
        │
   Interpreter           phoneme buffer → dictionary → word
        │                roll lookup → instant emit
        │                suffix → backspace + append
        │
 CGEvent (macOS)         inject text into focused application
 uinput passthrough (Linux) — passes non-rhe keys back to OS
                            — text injection coming (see Roadmap)
```

Purely event-driven. No polling, no timers, no key repeat. One
physical key change = one event = one state machine transition.

## Running

```
cargo run --release -- tutor     learn the chords
cargo run --release -- run       full engine (macOS menu bar only, for now)
cargo run --bin gen_briefs       regenerate roll assignments
cargo test                       verify everything
```

### macOS

Requires Input Monitoring permission (keyboard seizure via
IOHIDManager). Run with `sudo` or add your terminal to System
Settings → Privacy & Security → Input Monitoring.

### Linux

Grab keyboards via evdev and inject passthrough via uinput — the user
running rhe needs read access to `/dev/input/event*` and write access
to `/dev/uinput`. One-time setup:

```
sudo usermod -aG input $USER
echo 'KERNEL=="uinput", GROUP="input", MODE="0660"' \
    | sudo tee /etc/udev/rules.d/99-uinput.rules
sudo udevadm control --reload-rules
sudo modprobe -r uinput && sudo modprobe uinput
```

Log out and back in (or `newgrp input`) for the group change to
apply. Tutor works today on Linux; full `run` mode (text injection
into apps) is on the roadmap.

## Roadmap

Done:

- **IOHIDManager driver** (macOS) — raw HID, one event per key change
- **evdev driver** (Linux) — pre-xkb scancode grab + uinput passthrough
- **Interactive tutor** — step-by-step chord teaching with real-time
  key display, error recovery, brief/phoneme mode switching
- **Bench mode** (`rhe bench`) — measures chord speed per finger combo,
  averages across rounds, outputs ranking for mapping optimization
- **Frequency-optimized phoneme mapping** — 24 consonants + 15 vowels
  assigned by measured effort × phoneme frequency. No voicing pairs.
- **Roll system** — 496 word rolls ranked by `frequency × phonemes_saved`,
  right-only and two-hand slots ordered by measured effort
- **Suffix system** — 15 left-hand suffixes (-s, -ed, -ing, -ly, etc.)
  ordered by measured effort, inflected forms auto-excluded from rolls
- **Brief generator** — `gen_briefs` with pinned overrides, stemming,
  proper noun exclusion, value-based ranking

Short-term:

- **Linux text output** — libxkbcommon reverse-map + uinput injection
  for Latin output in the user's active layout (Dvorak/Colemak/any),
  with `Ctrl+Shift+U <hex> Enter` fallback for IPA and other unicode
  not representable in the current keymap. Unlocks `rhe run` on
  Linux.
- **Auto chord mapping** — `gen_map` reads bench timings + phoneme
  frequencies and generates `src/chord_map_data.rs` automatically.
  Users run bench, rebuild, mapping is personalized to their hands.
- **Number mode** — `word + mod` tap to enter. Ten physical finger
  positions (home row extended to include the inner-index stretch
  keys, QWERTY G + H) map 1-to-1 to digits 0–9. Mirrored 2-finger
  chords for negate, decimal point, thousands separator; mod-variants
  for `%`, `°`, `e`, `π`, etc.
- **Operators and symbols** — extended chord set for math, punctuation,
  and common programmer symbols, accessible via mode-switch chords.

Longer-term:

- **256-bit keymask** — generalize the chord representation from
  9-bit home-row to a full 256-bit HID usage bitmask. Lets users
  bind any chord on any physical key to any action (phoneme, brief,
  digit, custom text, mode switch).
- **User-configurable chord → action map** — config file so power
  users can rewire layouts without rebuilding.
- **libei migration** — replace the Linux uinput path with libei
  (freedesktop emulated-input protocol) once it stabilizes across
  major compositors. Cleaner text injection, no xkb reverse-map
  dance.

## Building

```
cargo build --release
cargo test
```

## License

[Mozilla Public License 2.0](LICENSE)
