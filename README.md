# Rhe

<p align="center">
  <img src="logo.png" width="300" alt="Rhe">
</p>

From Greek *rheo* — to flow. A first-principles phonetic chord engine.

10 keys. No finger movement. Phoneme-level input at the speed of thought.

## How it works

Your fingers rest on home row. They never leave. Rhe reads physical
key positions, not characters — it works with any OS keyboard layout.

```
Left hand (vowels)             Right hand (consonants)
pinky  ring  middle  index     index  middle  ring  pinky

              [mod]  [space]
              left    right
              thumb   thumb
```

Each phoneme is a chord: press fingers simultaneously, release. Right
hand = consonants (4 bits + mod = 5 bits). Left hand = vowels (4 bits).
Hands alternate and interleave for speed.

## The 9-bit key

```
Bits 0-3:  right hand fingers  (4 bits, 16 states)
Bits 4-7:  left hand fingers   (4 bits, 16 states)
Bit 8:     mod                 (right hand only)

Total:     512 chord keys
```

Space is not part of the chord key — it's the word boundary. Space
held = building a word. Space up = commit the word.

## Consonants (right hand)

24 consonants mapped to right hand. Mod = voiced pair (same mouth
position, same fingers, add mod).

```
Fingers     Plain         +Mod (voiced)
────────────────────────────────────────
I           t             d
M           s             z
R           k             g
P           p             b
I+M         n             m
I+R         r             dh (the)
M+R         l             [spare]
I+M+R       h             [spare]
I+P         f             v
M+P         w             [spare]
R+P         th (think)    [spare]
I+M+P       sh            zh (measure)
I+R+P       ch (church)   jh (judge)
M+R+P       ng            [spare]
I+M+R+P     y             [spare]
```

15 unvoiced + 9 voiced = 24 consonants. 6 mod slots spare.

## Vowels (left hand)

15 vowels mapped to left hand. No mod needed — 15 fits exactly.

```
Fingers     Sound
──────────────────────────
I           ah (but/sofa)
M           ih (sit)
R           eh (bed)
P           ae (cat)
I+M         ee (see)
I+R         aa (father)
M+R         ay (say)
I+M+R       er (her)
I+P         eye (my)
M+P         oh (go)
R+P         aw (law)
I+M+P       oo (too)
I+R+P       ow (how)
M+R+P       uh (book)
I+M+R+P     oy (boy)
```

4 bits = 15 non-zero states = 15 vowels. Complete General American coverage.

## Interleaved cadence

Hands alternate: left down, right down, left up, right up. One hand
loads the next chord while the other fires. Like a piano trill.

For "cat" (K + AE + T):

```
space  ===============================
right      ==K==          ==T==
left            ==AE==
fires           ^K   ^AE       ^T
```

Each hand fires on release. By the time one fires, the other is
already loaded. Effective speed per phoneme is the overlap window,
not the full press-release cycle.

Theoretical ceiling: ~325 WPM with perfect alternation, ~195 WPM
at 60% efficiency. Court reporter territory with briefs.

## Word input

**Space held = phoneme mode.** Chord phonemes in sequence while holding
space. Release space to commit the word. The engine looks up the
phoneme sequence in the dictionary and emits the word.

```
[space + K] [space + AE] [space + T] [release space] -> "cat "
```

**Space not held = brief mode.** Single chord, no space, instant output.
Both hands can press simultaneously. 511 possible briefs — enough for
the top 200 words (covering ~60% of all English text).

## Controls

```
Solo space tap        backspace (deletes last emitted word)
Mod tap + space held  undo last phoneme (before commit)
Mod alone             (reserved)
```

## The math

```
Phoneme slots:   39 used / 512 possible (with space held)
Brief slots:     511 possible (without space held)
Total gestures:  1,023

Coverage:
  39 phonemes = complete English phoneme inventory
  24 consonants (15 plain + 9 voiced via mod)
  15 vowels

Speed advantage:
  Zero finger travel (home row only)
  Chord parallelism (multiple fingers = one phoneme)
  Hand interleaving (load next while current fires)
  Brief shortcuts (common words = single chord)
```

## Architecture

```
IOHIDManager (macOS)    raw HID events, one per press/release, no repeats
    |
State machine           per-hand chord firing, interleaved
    |
Interpreter             phoneme buffer -> dictionary lookup -> word output
    |
CGEvent injection       text appears in focused application
```

The input driver seizes the keyboard at the HID level. One event per
physical key state change. Zero OS key repeat noise. Purely event-driven
— no polling, no timing loops.

## Status

Early development. Core engine, IOHIDManager driver, interactive tutor,
and phoneme-level output are functional. Dictionary lookup and brief
assignments in progress.

```
cargo run --release tutor    interactive typing tutor
cargo run --release run      full engine (menu bar)
cargo test                   run all tests
```

Requires macOS Accessibility / Input Monitoring permissions for keyboard
capture. Run with `sudo` or add terminal to Input Monitoring in
System Settings.

## Building

```
cargo build --release
cargo test
```

## License

[Mozilla Public License 2.0](LICENSE)
