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
(465) go to 3+ phoneme words. Effort ordering measured on real hands.

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

### Ordered briefs (homophone splits)

English is riddled with homophones — `to/too/two`, `no/know`,
`here/hear`, `right/write`, `for/four/fore`, `there/their`,
`read/red`, `by/buy/bye`, and so on. They share phoneme sequences,
so the phoneme path can only ever reach one of them (the most
frequent). Unordered briefs would give each word its own chord,
burning slots and obscuring the fact that they *are* the same
sound.

Ordered briefs put every member of a homophone set on the **same**
chord, distinguished by which key the user presses first.

**Finger difficulty for leading a chord** (easiest → hardest):

```
thumb < pinky < index < middle < ring
```

Thumb is the easiest lead because it's strong and independent.
Pinky has its own tendon and drops first cleanly. Ring is
tendon-coupled to middle and the hardest finger to move alone.

**Assignment rule**: easiest-available lead gets the most common
word, hardest-available gets the rare variant. For 3-way sets the
middle-difficulty finger gets the middle-frequency word.

Examples:

```
to / too / two       R-idx + R-mid + thumb
                       thumb-first  → "to"     (most common, easiest lead)
                       index-first  → "too"
                       middle-first → "two"    (rarest, hardest lead)

in / inn             R-mid + thumb
                       thumb-first  → "in"
                       middle-first → "inn"    (middle = hardest here)

there / their        R-mid + R-ring + thumb
                       thumb-first  → "there"
                       ring-first   → "their"  (ring = hardest)

for / four / fore    all four right-hand fingers
                       pinky-first  → "for"
                       middle-first → "four"
                       ring-first   → "fore"

no / know            L-mid + R-mid (symmetric split — hand-dominance rule)
                       right-lead   → "no"     (right hand = more common)
                       left-lead    → "know"
```

The symmetric same-finger-per-hand splits (no/know, here/hear,
right/write) use right-lead = more common instead of the difficulty
ranking, because both fingers in a symmetric pair are the same and
thus equally hard.

Curated in [`src/ordered_briefs_data.rs`](src/ordered_briefs_data.rs).
Each entry claims its chord slot — unordered briefs can't assign
to a claimed slot, so there's always one canonical word mapping
per chord. Every `gen_briefs` run also emits
[`data/homophones.txt`](data/homophones.txt) listing every CMU
collision set among the candidate pool, so new pairs are easy to
spot and add.

Tutor display: when the target is an ordered brief, the cell for
the first-down key renders at full brightness and the rest of the
target keys dim to the dot colour. The brighter cell is the
"press this one first" hint, no extra steps needed.

## Consonants

Mapped by frequency × measured chord effort. No voicing pairs —
pure speed optimization. The most common consonant (T) gets the
fastest chord (index). Assignments measured on real hands.

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
IOHIDManager (macOS)        raw key events (1 per state change, no repeats)
evdev grab (Linux)          — pre-xkb scancodes, layout-independent
        │
  State machine             word held → per-hand firing
        │                   word not held → all-zero firing (rolls)
        │
   Interpreter              phoneme buffer → dictionary → word
        │                   roll lookup → instant emit
        │                   suffix → backspace + append
        │
 CGEvent (macOS)            inject text into focused application
 uinput + xkb (Linux)       — reverse-map char → scancode+mods under
                              user's active xkb layout (Dvorak/Colemak/…)
                            — Ctrl+Shift+U <hex> Enter fallback for chars
                              outside the active layout (IPA, emoji)
```

Purely event-driven. No polling, no timers, no key repeat. One
physical key change = one event = one state machine transition.

## Running

```
cargo run --release              full engine (tray icon on macOS + Linux)
cargo run --bin gen_briefs       regenerate roll assignments
cargo test                       verify everything
```

The interactive tutor opens from the tray icon's right-click menu
("Open Tutor") — no separate CLI subcommand.

### macOS

Requires Accessibility permission. Add your terminal (or the rhe
binary) to System Settings → Privacy & Security → Accessibility.
No sudo needed.

One-time setup — disable the caps lock on-screen indicator and
firmware debounce (requires restart):

```
sudo defaults write /Library/Preferences/FeatureFlags/Domain/UIKit.plist \
    redesigned_text_cursor -dict-add Enabled -bool NO
```

Restart macOS. The caps lock indicator (blue arrow HUD) will no
longer appear, and caps lock responds instantly without the
built-in delay.

`rhe run` shows a menu bar item with a right-click menu: mode toggle
(`rhe` ↔ `keyboard`), fallback toggle (`Autospell` ↔ `IPA`), and
`Exit`. Tapping Caps Lock toggles rhe/keyboard mode. OS caps lock
is fully suppressed — rhe strips the alpha-shift flag from every
event, so caps lock only affects rhe's mode, never the OS text
layer. Media keys (volume, brightness, play/pause) pass thru
natively.

### Linux

Grab keyboards via evdev and inject passthru via uinput — the user
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
apply.

`rhe run` shows a tray icon (StatusNotifierItem) with a right-click
menu: mode toggle (`rhe` ↔ `keyboard`), fallback toggle
(`Autospell` ↔ `IPA`), and `Exit`. The tray works out of the box on
KDE, XFCE, Cinnamon, and MATE; on GNOME, install and enable
`gnome-shell-extension-appindicator`. Caps Lock also toggles
rhe/keyboard — a solo tap toggles, Caps+Esc quits (the up-edge is
only treated as a tap if no other key was pressed in between, so
combinations stay available). Unlike on macOS, the caps-lock LED
isn't a reliable indicator here because xkb actively reconciles it
against its own state — the tray icon's color is the visual mode
indicator on Linux.

Fallback mode choices: `Autospell` emits English-approximate ASCII
when a phoneme sequence isn't in the dictionary (any keyboard layout,
any terminal). `IPA` emits raw IPA unicode for every word, bypassing
the dictionary entirely — useful for linguistic work, but unicode
injection goes thru the GTK/Qt/IBus `Ctrl+Shift+U <hex> Enter`
sequence which looks like keyboard gibberish in terminals (`Ctrl+U`
= "kill line" there). Set `RHE_FALLBACK=ipa` to start in IPA mode;
set `RHE_UNICODE_FALLBACK=off` to drop unmapped chars silently
instead of using the Ctrl+Shift+U path.

## Roadmap

Done:

- **IOHIDManager driver** (macOS) — raw HID, one event per key change
- **evdev driver** (Linux) — pre-xkb scancode grab + uinput passthru
- **Interactive tutor** — GUI drill window opened from the tray
  menu, with real-time key display, error recovery, brief/phoneme
  mode switching, adaptive cell labels (each cell shows what its
  press would emit in the current chord), and a scrolling sentence
  context line above the target word
- **Frequency-optimized phoneme mapping** — 24 consonants + 15 vowels
  assigned by measured effort × phoneme frequency. No voicing pairs.
- **Roll system** — 496 word rolls ranked by `frequency × phonemes_saved`,
  right-only and two-hand slots ordered by measured effort
- **Suffix system** — 15 left-hand suffixes (-s, -ed, -ing, -ly, etc.)
  ordered by measured effort, inflected forms auto-excluded from rolls
- **Brief generator** — `gen_briefs` with stemming, proper-noun
  exclusion, and a unified greedy pass ranking candidate words by
  `frequency × (phonemes - 1)` against slots sorted by ergonomic
  effort. Writes `src/preferences/briefs_data.rs`.
- **Curated candidates file** — `data/brief_candidates.txt` is the
  editable source of truth for which words get briefs. Each line
  carries inline auto-reasons in its comment column: suffix
  decomposition (`+ing→base(freq) defer/keep`), homophone collision,
  sidecar-category exclusion, or ordered-brief slot claim. Comment
  out a line to drop the word; un-comment to bring it back.
  Re-runs of `gen_briefs` refresh ranks/values/reasons but preserve
  user `#`s, custom-added words, and any user notes after a ` ; `
  separator.
- **Sidecar excludes file** — `data/brief_excludes.txt` contains
  categorized word lists (gender, religion, proper nouns by person /
  place, nationality, title, profanity, stage directions, informal
  fragments, number-mode aliases, abbreviations) that auto-`#` the
  matching candidates with `excluded: <category>` reasons. Per-
  category disable by commenting the section header; per-word
  opt-back-in by removing the word from the file. Lets the default
  brief set stay free of statehood / religious / racial / gender
  bias and proper-noun clutter, with each install free to opt back
  in selectively.
- **Homophone collision report** — `gen_briefs` emits
  `data/homophones.txt` every run, listing every CMU phoneme-sequence
  set that has multiple frequency-listed words. Scoped to sets
  reachable via a candidate word so it stays useful, not noisy.
- **Ordered briefs** — chord slots can carry multiple words
  distinguished by which key lands first. 2-way homophones
  (no/know, here/hear, right/write) use symmetric same-finger-per-
  hand split chords; 3-way sets (to/too/two, for/four/fore) use
  single-hand 3- or 4-finger chords with outer-left / outer-right /
  center leads. The mechanism extends beyond homophones to:
  morphological pairs (go/going, have/having, work/working, …),
  unreachable-homophone allocations (peace/piece, weather/whether,
  cell/sell, led/lead, role/roll, site/sight) so the lower-freq
  member is typeable at all, and the spelled-number bundle for
  one/won. Curated in `src/layout/ordered_briefs.rs`; the tutor
  brightens the first-down cell so the ordering is obvious at a
  glance.
- **Compound family pattern** — `thing` and its prefix compounds
  (something / anything / nothing / everything) share a base R-only
  chord (R-IDX+R-MID+R-RING+R-thumb), with each prefix modifier
  adding a single left finger: `some-` → L-MID, `any-` → L-RING,
  `no-` → L-PINKY, `every-` → L-IDX. The `one`-family
  (someone/anyone/everyone) extends the same convention. Once the
  prefix→finger map is in muscle memory, every compound is
  reachable with one chord and the family stays mentally unified.
- **Spelled-zero pristine gesture** — `word + mod-tap + word-up`
  (number mode entered with no digit chord between) emits the
  spelled "zero". No chord slot consumed; the empty-mode-exit case
  is repurposed as the shortcut. Number-form transforms still
  apply (the emit arms `last_number` so L-hand chords can transform
  to "zeroth" etc.).
- **Linux text output** — libxkbcommon reverse-map + uinput injection
  for Latin output in the user's active layout (Dvorak/Colemak/any),
  with `Ctrl+Shift+U <hex> Enter` fallback for IPA and other unicode
  not representable in the current keymap.
- **Linux `rhe run`** — full engine on Linux (evdev grab + uinput
  injection). Caps Lock tap toggles rhe enabled/disabled, Caps+Esc
  quits. Esc alone passes thru to the focused app.
- **Cross-platform tray menu** — StatusNotifierItem on Linux / native
  NSStatusItem on macOS via `tray-icon`. Right-click for mode toggle
  (rhe ↔ keyboard), fallback toggle (Autospell ↔ IPA), and exit.
  Fully event-driven — the tao event loop sleeps until a menu click
  or a caps-tap proxy nudge fires. Caps-lock LED tracks mode on
  macOS (off = rhe, on = keyboard); Linux uses the tray icon's color
  since xkb fights direct LED writes.
- **Random practice text** — tutor pulls Wikipedia article extracts
  via the MediaWiki random-article API, double-buffered: one batch
  serves the current drill while the next prefetches in the
  background, so wraparounds swap instantly. Strips brackets,
  parentheticals, non-ASCII; falls back to bundled Alice in
  Wonderland if offline. Random starting sentence each run.
- **Number mode** — `word + mod` tap to enter. Ten home-row +
  inner-index positions (G/H included) map 1-to-1 to digits 0–9;
  mod-variants give symbols (`+`, `-`, `*`, `/`, `%`, etc.). Word
  release commits, then six L-hand chords transform the just-emitted
  integer into spelled cardinal / ordinal / multiplier / group /
  fraction / prefix forms.
- **Six physical layouts** — narrow / medium / wide × right- or
  left-dominant. `CURRENT` in `src/layout/keyboard.rs` picks the
  active one at compile time; the rest of the engine is agnostic.
- **Repository reorg** — user-tunable mappings live under
  `src/layout/` (chords, keyboard, briefs, ordered_briefs,
  suffixes, numbers, number_forms); GUI + drill state under
  `src/tutor/` (drill, wiki, ui/compositor + ui/drawing +
  ui/text_rasterizing + ui/theme). Single `theme.rs` is the source
  of truth for every colour. Brief loader and phoneme-dictionary
  loader live at crate root (`src/briefs.rs`, `src/phoneme_dict.rs`)
  — they build runtime tables from the layout/ data, distinct from
  the data itself.

Short-term:

- **Auto chord mapping** — `gen_map` reads bench timings + phoneme
  frequencies and generates `src/chord_map_data.rs` automatically.
  Users run bench, rebuild, mapping is personalized to their hands.
- **Operators and symbols beyond the number-mode set** — extended
  chord set for punctuation and common programmer symbols
  reachable from a non-number sub-mode, so they don't fight for
  digit slots.
- **Brief generator improvements** — rework the brief-selection
  scoring to (a) exclude natural-split 2-phoneme words where the
  brief gesture equals the phoneme gesture (wasted slot), (b) rank
  chord slots by ergonomic cost (finger count + tendon-conflict
  skip patterns), and (c) match value-ranked words to cost-ranked
  slots pairwise rather than bit-order.
- **Runtime layout switching** — `CURRENT` in `layout.rs` is
  compile-time; expose a tray-menu picker (or config file) so
  users can swap between the six bundled layouts without
  recompiling.

Longer-term:

- **Full down-order chord slots** — a first cut shipped as "ordered
  briefs" (see Done): we track the *first* key to go down and use
  that to disambiguate between words sharing a chord. The full
  feature would carry the entire down-order permutation so each
  N-finger chord becomes N! slots (2-finger → 2, 3-finger → 6,
  4-finger → 24). Down-edges only — release ordering stays
  unconstrained since users don't control it consciously. Would let
  a single chord shape hold 6+ distinct words ranked by roll order
  rather than just 2.
- **256-bit keymask** — generalize the chord representation from
  9-bit home-row to a full 256-bit HID usage bitmask. Lets users
  bind any chord on any physical key to any action (phoneme, brief,
  digit, custom text, mode switch).
- **User-configurable chord → action map** — config file so power
  users can rewire layouts without rebuilding.
- **Linux layout hot-reload** — detect xkb layout changes at runtime
  (currently the reverse map is built once at startup). Matters if
  the user switches layouts mid-session.
- **libei migration** — replace the Linux uinput path with libei
  (freedesktop emulated-input protocol) once it stabilizes across
  major compositors. Cleaner text injection, no xkb reverse-map
  dance.

## Building

```
cargo build --release
cargo test
```

## On building this with an LLM

Most commits in this repo land with `Co-Authored-By: Claude Opus`. That tag is the conventional disclosure, but it doesn't quite capture what's going on.

The structural thing first: wisdom needs stakes and continuity, and an LLM has neither. Knowledge is storable — text in, text out — so a model trained on enough of it ends up with broad pattern access. Wisdom is what happens when a continuous self is *shaped by consequences over time*. Claude doesn't have a continuous self. Each conversation starts fresh. Nothing it says has cost. It can't have a hunch ripen, can't get burned and remember, can't accumulate the kind of taste that comes from living with its own bad calls.

The closest human analogy is the brilliant prodigy who's read every book but lived thru nothing. They can quote every wise person but don't know what to do when their parent dies, when they fall in love, when they fail at something they cared about. The knowledge is in there; it just hasn't been metabolized into judgment by being a person who needed it.

Which makes the collaboration asymmetric in a useful way. I bring vision, judgment, and the willingness to live with bad calls. Claude brings:

- **Consistency at scale.** Applying "exclude gendered terms" across 54 words in `data/brief_excludes.txt` is 30 seconds of LLM time vs. an hour of mine, plus the miss on `gentleman` because it slipped past the obvious `he/him/his` cluster. The convention is mine; the per-word execution is Claude's.
- **Unbiased categorization.** Default-excluding proper nouns and gendered titles isn't itself neutral, but the categorization is. An LLM treats `kim`, `jane`, `daniel`, and `william` the same way — no curator's blind spots about whose names "feel like proper nouns." The brief set is fairer because a tireless outsider helped curate it, not because I'm careful.
- **Mechanical cleanup.** The 44-file doc-comment unwrap, the `preferences/` → `layout/` rename touching ~50 files, the `table_gen.rs` → `phoneme_dict.rs` rename plus parser dedupe — the kind of work I'd put off forever. With Claude they happen the moment I notice them.
- **A second pair of eyes on judgment calls.** When I rewrite an interpreter branch, Claude catches that the spelled-zero change broke `ordinal_not_triggered_after_symbol` because of a `last_number` flag interaction. The bug is mine; the fast feedback is the LLM's.

A strange consequence of all this: an LLM can *sound* wise without being wise. It can recite what wise people said with high fidelity. The way I tell the difference is by my own taste — the words alone don't transmit the substance. So our exchanges are asymmetric in a precise way: Claude supplies pattern, I supply taste. Whether what it says is *good* is mostly a function of my judgment on its output. The pattern-recognition is real. The caring isn't. And caring is most of what wisdom is.

Also worth flagging — "infinite knowledge" overshoots. LLMs don't have perfect recall; they reconstruct probabilistically and will confidently confabulate when reaching past coverage. It's broad pattern access, not encyclopedic memory. The hole-shapes are different from a human's, which is why an LLM is useful as a *complement* rather than a replacement.

So: not co-authored in the way two engineers co-author a paper. More like… built with a tireless assistant that never gets tired and never has an agenda but also won't stop me from making a questionable choice when I want to. The project is what it is because of choices I made; it's *finished* — unified, unbiased, clean — because Claude carried the pattern across.

If you're building something opinionated and have a clear vision but limited tolerance for the mechanical work of seeing it thru, an LLM is a force multiplier that conventional automation can't match. It applies pattern. It outlines tradeoffs. It catches consistency bugs. It doesn't decide what matters — and treating it as if it could is how projects end up with the *texture* of wisdom but not the substance.

## License

[Mozilla Public License 2.0](LICENSE)
