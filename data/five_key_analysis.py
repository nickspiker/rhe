#!/usr/bin/env python3
"""
5-key per hand analysis.
Fingers: Pinky, Ring, Middle, Index-home, Index-stretch
31 combos per hand. Find optimal consonant assignment with minimal collisions.
"""

import re
from collections import Counter
from itertools import combinations, permutations

VOWELS = {'AA','AE','AH','AO','AW','AY','EH','ER','EY','IH','IY','OW','OY','UH','UW'}

def load_freq(path):
    freq = {}
    with open(path) as f:
        for line in f:
            parts = line.strip().split()
            if len(parts) == 2:
                word, count = parts[0].lower(), int(parts[1])
                if word not in freq:
                    freq[word] = count
    return freq

def load_cmudict(path):
    entries = {}
    with open(path) as f:
        for line in f:
            if line.startswith(';;;'):
                continue
            parts = line.strip().split()
            if len(parts) < 2:
                continue
            word = re.sub(r'\(\d+\)$', '', parts[0].lower())
            phonemes = [re.sub(r'\d', '', p) for p in parts[1:]]
            if word not in entries:
                entries[word] = phonemes
    return entries

def syllabify(phonemes):
    vowel_positions = [i for i, p in enumerate(phonemes) if p in VOWELS]
    if not vowel_positions:
        return []
    syllables = []
    for si, vi in enumerate(vowel_positions):
        if si == 0:
            onset_start = 0
        else:
            prev_vi = vowel_positions[si - 1]
            consonants_between = phonemes[prev_vi + 1:vi]
            if len(consonants_between) <= 1:
                onset_start = vi - len(consonants_between)
            else:
                onset_start = prev_vi + 2
        onset = tuple(phonemes[onset_start:vi])
        nucleus = phonemes[vi]
        if si == len(vowel_positions) - 1:
            coda = tuple(phonemes[vi + 1:])
        else:
            next_vi = vowel_positions[si + 1]
            consonants_between = phonemes[vi + 1:next_vi]
            if len(consonants_between) <= 1:
                coda = ()
            else:
                coda = (phonemes[vi + 1],)
        syllables.append((onset, nucleus, coda))
    return syllables

def main():
    freq = load_freq('/Users/nick/code/rhe/data/en_freq.txt')
    cmudict = load_cmudict('/Users/nick/code/rhe/data/cmudict.dict')

    print("=" * 70)
    print("  5-KEY PER HAND ANALYSIS")
    print("=" * 70)

    # Key positions: Pinky(P), Ring(R), Middle(M), Index-home(I), Index-stretch(X)
    # Bits: P=4, R=3, M=2, I=1, X=0
    # 31 combos per hand

    fingers = ['X', 'I', 'M', 'R', 'P']  # bit positions 0-4

    combos_by_size = {}
    for r in range(1, 6):
        combos_by_size[r] = list(combinations(range(5), r))

    print(f"\n  Combos per hand:")
    for r in range(1, 6):
        print(f"    {r}-key: {len(combos_by_size[r])}")
    total_combos = sum(len(v) for v in combos_by_size.values())
    print(f"    Total: {total_combos}")

    # Comfort model for 5 keys
    # Positions: 0=X(index stretch), 1=I(index home), 2=M(middle), 3=R(ring), 4=P(pinky)
    finger_names = {0: 'X(stretch)', 1: 'I(index)', 2: 'M(middle)', 3: 'R(ring)', 4: 'P(pinky)'}

    single_effort = {0: 1.3, 1: 1.0, 2: 1.1, 3: 1.5, 4: 2.0}

    # Two-key effort based on adjacency and finger strength
    def chord_effort(bits):
        fingers_used = [i for i in range(5) if bits & (1 << i)]
        if len(fingers_used) == 1:
            return single_effort[fingers_used[0]]
        # Base: sum of individual efforts * 0.6 + stretch penalty
        base = sum(single_effort[f] for f in fingers_used) * 0.6
        # Add penalty for non-adjacent fingers
        for i in range(len(fingers_used)):
            for j in range(i+1, len(fingers_used)):
                gap = abs(fingers_used[j] - fingers_used[i])
                if gap > 1:
                    base += 0.3 * (gap - 1)
        return base

    # Generate all 31 combos with effort scores
    all_combos = []
    for bits in range(1, 32):
        fingers_used = [finger_names[i] for i in range(5) if bits & (1 << i)]
        effort = chord_effort(bits)
        all_combos.append((bits, fingers_used, effort))

    all_combos.sort(key=lambda x: x[2])

    print(f"\n  All 31 combos ranked by effort:")
    print(f"  {'Rank':<5s} {'Bits':<8s} {'Fingers':<40s} {'Effort':>6s}")
    for rank, (bits, fingers_used, effort) in enumerate(all_combos):
        print(f"  {rank+1:<5d} {bits:05b}    {', '.join(fingers_used):<40s} {effort:>6.2f}")

    # Now: combined consonant frequency
    consonant_freq = Counter()
    for word, count in freq.items():
        if word not in cmudict:
            continue
        for p in cmudict[word]:
            if p not in VOWELS:
                consonant_freq[p] += count

    # Filter to real consonants
    real_consonants = ['T', 'N', 'S', 'R', 'D', 'L', 'M', 'K', 'DH', 'W',
                       'Z', 'Y', 'HH', 'B', 'P', 'F', 'V', 'G', 'NG',
                       'SH', 'TH', 'JH', 'CH', 'ZH']

    print(f"\n  24 consonants by frequency:")
    for i, c in enumerate(sorted(real_consonants, key=lambda x: -consonant_freq.get(x, 0))):
        print(f"    {i+1:2d}. {c:4s}: {consonant_freq.get(c, 0):>12,d}")

    # With 31 combos we can fit all 24 consonants with 7 to spare!
    print(f"\n  31 combos - 24 consonants = 7 spare slots")
    print(f"  Spare slots could be: common clusters, modifiers, or unused")

    # Now find optimal assignment: top 5 single keys
    # With 5 single keys, we can test which 5 consonants minimize collisions
    candidates = ['T', 'N', 'S', 'R', 'D', 'L', 'M', 'K', 'DH', 'W', 'Z', 'Y', 'HH', 'B']

    # Build cluster frequency map
    all_clusters = Counter()
    total_syllables = 0
    for word, count in freq.items():
        if word not in cmudict:
            continue
        phonemes = cmudict[word]
        syllables = syllabify(phonemes)
        for onset, nucleus, coda in syllables:
            total_syllables += count
            all_phonemes = list(onset) + list(coda)
            for i in range(len(all_phonemes)):
                for j in range(i+1, len(all_phonemes)):
                    pair = tuple(sorted([all_phonemes[i], all_phonemes[j]]))
                    all_clusters[pair] += count

    print(f"\n{'═' * 70}")
    print(f"  OPTIMAL 5-KEY SINGLE ASSIGNMENTS (minimize collisions)")
    print(f"{'═' * 70}")

    best = []
    for combo in combinations(candidates, 5):
        collision_total = 0
        for i in range(5):
            for j in range(i+1, 5):
                pair = tuple(sorted([combo[i], combo[j]]))
                collision_total += all_clusters.get(pair, 0)
        best.append((collision_total, combo))

    best.sort()

    print(f"\n  Top 20 lowest-collision 5-key assignments:")
    print(f"  {'Rank':<5s} {'Key1':<5s} {'Key2':<5s} {'Key3':<5s} {'Key4':<5s} {'Key5':<5s} {'Collision':>12s} {'%':>8s}")
    seen_sets = set()
    for rank, (coll, combo) in enumerate(best[:20]):
        pct = coll / total_syllables * 100
        print(f"  {rank+1:<5d} {combo[0]:<5s} {combo[1]:<5s} {combo[2]:<5s} {combo[3]:<5s} {combo[4]:<5s} {coll:>12,d} {pct:>7.2f}%")

    # Check specifically L, M, N, R + one more
    print(f"\n  L, M, N, R + one more:")
    lmnr_plus = []
    for extra in candidates:
        if extra in ('L', 'M', 'N', 'R'):
            continue
        combo = tuple(sorted(['L', 'M', 'N', 'R', extra]))
        collision_total = 0
        for i in range(5):
            for j in range(i+1, 5):
                pair = tuple(sorted([combo[i], combo[j]]))
                collision_total += all_clusters.get(pair, 0)
        lmnr_plus.append((collision_total, extra, combo))

    lmnr_plus.sort()
    print(f"  {'Extra':<6s} {'Collision':>12s} {'%':>8s}")
    for coll, extra, combo in lmnr_plus:
        pct = coll / total_syllables * 100
        print(f"  {extra:<6s} {coll:>12,d} {pct:>7.2f}%")

    # Full encoding space
    print(f"\n{'═' * 70}")
    print(f"  FULL ENCODING SPACE")
    print(f"{'═' * 70}")
    print(f"  Per hand: 31 combos + empty = 32 states")
    print(f"  Right × Left × 4 modes = 32 × 32 × 4 = {32*32*4:,}")
    print(f"  With space (word mode):    {32*32*4:,} slots")
    print(f"  Without space (brief/sym): {32*32*4:,} slots")
    print(f"  Total:                     {32*32*4*2:,} slots")
    print(f"")

    # How many consonants fit without any modifier?
    print(f"  31 combos = 24 consonants + 7 spare")
    print(f"  No Ctrl needed for consonants!")
    print(f"")

    # Vowel encoding: need 15 vowels
    # 4 modes = only 4, need multiplier
    # Options:
    print(f"  VOWEL ENCODING OPTIONS:")
    print(f"  A) 4 modes × 2 Ctrl states = 8 (not enough)")
    print(f"  B) Use 7 spare combos as vowel modifiers")
    print(f"     4 modes = 4 base vowels")
    print(f"     4 modes × with-modifier = 4 more = 8 total")
    print(f"     Still need 7 more...")
    print(f"  C) Use one finger as dedicated vowel key")
    print(f"     e.g., Stretch key = vowel group shift")
    print(f"     4 modes × 2 stretch states = 8 slots")
    print(f"     Still not enough for 15...")
    print(f"  D) Ctrl = vowel extension only (not consonants)")
    print(f"     4 modes × 2 Ctrl × 2 stretch? = 16 slots")
    print(f"     That works! Ctrl+stretch gives 4 groups of 4")
    print(f"  E) 4 modes × 4 thumb combos (space+ctrl: none/space/ctrl/both)")
    print(f"     = 16 slots. Space is held for word mode anyway...")
    print(f"     Wait: space is already being used as word boundary.")
    print(f"     Within a word (space held), thumb state is always 'space=down'")
    print(f"     So we only have Ctrl as a free thumb within a word.")
    print(f"     4 modes × 2 Ctrl = 8 vowel slots (not enough)")
    print(f"  F) Ctrl is free within a word. But we could also use")
    print(f"     the STRETCH key on one or both hands as vowel info.")
    print(f"     If stretch-key is NOT a consonant but a vowel modifier:")
    print(f"     Right-stretch × Left-stretch × 4 modes = 2 × 2 × 4 = 16")
    print(f"     That gives 16 vowel slots with NO Ctrl needed!")

    # Let's explore option F more carefully
    print(f"\n{'─' * 70}")
    print(f"  OPTION F: Stretch keys as vowel selectors")
    print(f"{'─' * 70}")
    print(f"  If we reserve both stretch keys (i and d) for vowel selection:")
    print(f"  - Each hand has 4 home keys = 15 consonant combos")
    print(f"  - 15 is NOT enough for 24 consonants (need Ctrl again)")
    print(f"")
    print(f"  If we reserve ONE stretch key (say left) for vowel:")
    print(f"  - Right hand: 5 keys = 31 consonant combos (onset)")
    print(f"  - Left hand: 4 home keys = 15 consonant combos (coda)")
    print(f"  - Left stretch × 4 modes = 2 × 4 = 8 vowel slots")
    print(f"  - Still only 8 vowels, and left hand only has 15 combos")
    print(f"")
    print(f"  ACTUALLY: with 5 keys per hand = 31 combos each")
    print(f"  24 consonants = 24 of the 31 combos used")
    print(f"  7 spare combos per hand")
    print(f"  Right hand spare combos DURING a chord could indicate vowel")
    print(f"  But you can't chord a consonant AND a vowel-modifier simultaneously")
    print(f"  on the same hand... or CAN you? The 7 spares are specific combos.")
    print(f"  This gets complicated.")
    print(f"")
    print(f"  SIMPLEST: Ctrl for vowels. 4 modes × 2 Ctrl = 8 base vowels (84%)")
    print(f"  For the remaining 7 vowels: Ctrl + a specific finger combo?")
    print(f"  Or: just accept 8 vowels cover 84% and merge similar vowels.")

    # Vowel frequency
    vowel_freq = Counter()
    for word, count in freq.items():
        if word not in cmudict:
            continue
        for p in cmudict[word]:
            if p in VOWELS:
                vowel_freq[p] += count

    total_v = sum(vowel_freq.values())
    print(f"\n  Vowel frequency (cumulative):")
    cum = 0
    for i, (v, count) in enumerate(vowel_freq.most_common()):
        pct = count / total_v * 100
        cum += pct
        marker = ""
        if i == 7:
            marker = " ← 8 vowel cutoff"
        if i == 3:
            marker = " ← 4 vowel cutoff (modes only)"
        print(f"    {i+1:2d}. {v:4s} {pct:5.1f}% (cum: {cum:5.1f}%){marker}")

if __name__ == '__main__':
    main()
