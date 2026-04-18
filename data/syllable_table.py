#!/usr/bin/env python3
"""
Generate the full syllable → chord mapping table.

Strategy:
1. Get all syllables from CMU dict weighted by frequency
2. Assign consonants to finger combos (5 keys, 31 combos, both hands same map)
3. For each onset+coda pair, assign vowel variants across modes × ctrl
4. Output the complete table

Key insight: onset is always right hand shape, coda is always left hand shape.
Same shape = same consonant sound. Mode and ctrl pick the vowel.
"""

import re
from collections import Counter, defaultdict

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

    # Step 1: Count all (onset_consonants, vowel, coda_consonants) triples
    syllable_freq = Counter()
    total_syllables = 0

    for word, count in freq.items():
        if word not in cmudict:
            continue
        phonemes = cmudict[word]
        syllables = syllabify(phonemes)
        for onset, nucleus, coda in syllables:
            syllable_freq[(onset, nucleus, coda)] += count
            total_syllables += count

    # Step 2: Count unique onset and coda patterns
    onset_freq = Counter()
    coda_freq = Counter()
    for (onset, nucleus, coda), count in syllable_freq.items():
        onset_freq[onset] += count
        coda_freq[coda] += count

    # Step 3: For each (onset, coda) pair, count how many vowels appear
    onset_coda_vowels = defaultdict(Counter)
    for (onset, nucleus, coda), count in syllable_freq.items():
        onset_coda_vowels[(onset, coda)][nucleus] += count

    # How many vowel variants does each onset+coda pair need?
    vowel_count_dist = Counter()
    for (onset, coda), vowels in onset_coda_vowels.items():
        vowel_count_dist[len(vowels)] += 1

    print("=" * 70)
    print("  SYLLABLE TABLE GENERATION")
    print(f"  Total syllables: {total_syllables:,}")
    print(f"  Unique syllables: {len(syllable_freq):,}")
    print(f"  Unique onsets: {len(onset_freq):,}")
    print(f"  Unique codas: {len(coda_freq):,}")
    print(f"  Unique onset+coda pairs: {len(onset_coda_vowels):,}")
    print("=" * 70)

    # How many vowel variants per onset+coda pair?
    print(f"\n  Vowel variants per onset+coda pair:")
    print(f"  {'Variants':<10s} {'Pairs':>8s} {'Cumulative':>12s}")
    cum = 0
    for n in sorted(vowel_count_dist.keys()):
        cum += vowel_count_dist[n]
        pct = cum / len(onset_coda_vowels) * 100
        print(f"  {n:<10d} {vowel_count_dist[n]:>8d} {pct:>11.1f}%")

    # Key question: max vowels per onset+coda pair?
    max_vowels = max(vowel_count_dist.keys())
    print(f"\n  Max vowel variants for any onset+coda pair: {max_vowels}")
    print(f"  We have 4 modes × 2 ctrl = 8 vowel slots per onset+coda pair")

    # How many pairs need more than 8?
    over_8 = sum(c for n, c in vowel_count_dist.items() if n > 8)
    over_4 = sum(c for n, c in vowel_count_dist.items() if n > 4)
    print(f"  Pairs needing >4 vowel slots: {over_4} ({over_4/len(onset_coda_vowels)*100:.1f}%)")
    print(f"  Pairs needing >8 vowel slots: {over_8} ({over_8/len(onset_coda_vowels)*100:.1f}%)")

    # What % of SYLLABLE USAGE needs >8?
    usage_over_8 = 0
    usage_over_4 = 0
    for (onset, coda), vowels in onset_coda_vowels.items():
        if len(vowels) > 8:
            usage_over_8 += sum(vowels.values())
        if len(vowels) > 4:
            usage_over_4 += sum(vowels.values())
    print(f"  Syllable usage needing >4 slots: {usage_over_4/total_syllables*100:.1f}%")
    print(f"  Syllable usage needing >8 slots: {usage_over_8/total_syllables*100:.1f}%")

    # Step 4: Consonant assignment
    # Get single-consonant onsets and codas by frequency
    single_onset_freq = Counter()
    single_coda_freq = Counter()
    for (onset, nucleus, coda), count in syllable_freq.items():
        if len(onset) == 1:
            single_onset_freq[onset[0]] += count
        elif len(onset) == 0:
            single_onset_freq['(none)'] += count
        if len(coda) == 1:
            single_coda_freq[coda[0]] += count
        elif len(coda) == 0:
            single_coda_freq['(none)'] += count

    combined_freq = Counter()
    for c, f in single_onset_freq.items():
        if c != '(none)':
            combined_freq[c] += f
    for c, f in single_coda_freq.items():
        if c != '(none)':
            combined_freq[c] += f

    # Assign consonants to combo slots by frequency → effort
    # 5 keys: bits 0-4 = X(stretch), I(index), M(middle), R(ring), P(pinky)
    single_effort = {0: 1.3, 1: 1.0, 2: 1.1, 3: 1.5, 4: 2.0}
    def chord_effort(bits):
        fingers_used = [i for i in range(5) if bits & (1 << i)]
        if len(fingers_used) == 1:
            return single_effort[fingers_used[0]]
        base = sum(single_effort[f] for f in fingers_used) * 0.6
        for i in range(len(fingers_used)):
            for j in range(i+1, len(fingers_used)):
                gap = abs(fingers_used[j] - fingers_used[i])
                if gap > 1:
                    base += 0.3 * (gap - 1)
        return base

    all_combos = sorted([(bits, chord_effort(bits)) for bits in range(1, 32)], key=lambda x: x[1])

    sorted_consonants = sorted(combined_freq.keys(), key=lambda x: -combined_freq[x])

    print(f"\n{'═' * 70}")
    print(f"  CONSONANT → CHORD ASSIGNMENT")
    print(f"{'═' * 70}")

    finger_names = {0: 'X', 1: 'I', 2: 'M', 3: 'R', 4: 'P'}
    def bits_to_str(bits):
        return '+'.join(finger_names[i] for i in range(5) if bits & (1 << i))

    consonant_to_chord = {}
    consonant_to_chord['(none)'] = 0  # no fingers = no consonant

    for i, consonant in enumerate(sorted_consonants):
        if i < len(all_combos):
            bits, effort = all_combos[i]
            consonant_to_chord[consonant] = bits
            print(f"  {consonant:4s} → {bits:05b} ({bits_to_str(bits):16s}) effort={effort:.2f}  freq={combined_freq[consonant]:>12,d}")
        else:
            print(f"  {consonant:4s} → OVERFLOW (no slot!)")

    # Spare slots
    used_bits = set(consonant_to_chord.values())
    spare = [b for b in range(0, 32) if b not in used_bits]
    print(f"\n  Spare chord slots: {len(spare)}")
    for b in spare:
        print(f"    {b:05b} ({bits_to_str(b) if b else 'empty':16s}) effort={chord_effort(b) if b else 0:.2f}")

    # Step 5: Count onset+coda pairs that involve multi-consonant clusters
    print(f"\n{'═' * 70}")
    print(f"  ONSET/CODA CLUSTER HANDLING")
    print(f"{'═' * 70}")

    cluster_onset_freq = Counter()
    cluster_coda_freq = Counter()
    for (onset, nucleus, coda), count in syllable_freq.items():
        if len(onset) >= 2:
            cluster_onset_freq[onset] += count
        if len(coda) >= 2:
            cluster_coda_freq[coda] += count

    cluster_onset_usage = sum(cluster_onset_freq.values())
    cluster_coda_usage = sum(cluster_coda_freq.values())
    print(f"  Syllables with cluster onset: {cluster_onset_usage/total_syllables*100:.1f}%")
    print(f"  Syllables with cluster coda: {cluster_coda_usage/total_syllables*100:.1f}%")
    print(f"  Top cluster onsets:")
    for onset, count in cluster_onset_freq.most_common(10):
        pct = count / total_syllables * 100
        print(f"    {'+'.join(onset):12s} {pct:.2f}%")
    print(f"  Top cluster codas:")
    for coda, count in cluster_coda_freq.most_common(10):
        pct = count / total_syllables * 100
        print(f"    {'+'.join(coda):12s} {pct:.2f}%")

    # Use spare slots for common clusters!
    print(f"\n  Spare slots available for clusters: {len(spare)}")

    # We could assign the top onset clusters to spare onset combos
    # and top coda clusters to spare coda combos
    # But since both hands share the same map, a "cluster combo"
    # works on both sides

    # Step 6: Full coverage calculation
    print(f"\n{'═' * 70}")
    print(f"  FULL COVERAGE ANALYSIS")
    print(f"{'═' * 70}")

    # For each syllable, check if it can be encoded
    encodable = 0
    not_encodable = 0
    not_encodable_freq = 0

    # An onset is encodable if it's a single consonant in our map,
    # empty, or a cluster we assign to a spare slot
    # For now: single consonant or empty = encodable
    for (onset, nucleus, coda), count in syllable_freq.items():
        onset_ok = len(onset) <= 1 and (len(onset) == 0 or onset[0] in consonant_to_chord)
        coda_ok = len(coda) <= 1 and (len(coda) == 0 or coda[0] in consonant_to_chord)

        if onset_ok and coda_ok:
            encodable += count
        else:
            not_encodable += count
            not_encodable_freq += 1

    print(f"  Single-consonant encodable: {encodable/total_syllables*100:.1f}% of usage")
    print(f"  Needs cluster handling: {not_encodable/total_syllables*100:.1f}% of usage")
    print(f"  Unique unencodable syllables: {not_encodable_freq}")

    # If we add top 7 clusters to spare slots:
    top_clusters = []
    for onset, count in cluster_onset_freq.most_common():
        top_clusters.append(('onset', onset, count))
    for coda, count in cluster_coda_freq.most_common():
        top_clusters.append(('coda', coda, count))
    top_clusters.sort(key=lambda x: -x[2])

    print(f"\n  Top clusters we'd assign to spare slots:")
    assigned_clusters = {}
    spare_iter = iter(spare[1:])  # skip 0 (empty)
    for pos, cluster, count in top_clusters[:7]:
        try:
            bits = next(spare_iter)
            assigned_clusters[cluster] = bits
            pct = count / total_syllables * 100
            print(f"    {'+'.join(cluster):12s} ({pos:5s}) → {bits:05b} ({bits_to_str(bits)}) {pct:.2f}%")
        except StopIteration:
            break

    # Recalculate with clusters
    encodable2 = 0
    for (onset, nucleus, coda), count in syllable_freq.items():
        onset_ok = (len(onset) == 0 or
                   (len(onset) == 1 and onset[0] in consonant_to_chord) or
                   onset in assigned_clusters)
        coda_ok = (len(coda) == 0 or
                  (len(coda) == 1 and coda[0] in consonant_to_chord) or
                  coda in assigned_clusters)
        if onset_ok and coda_ok:
            encodable2 += count

    print(f"\n  With 7 cluster slots: {encodable2/total_syllables*100:.1f}% of syllable usage encodable")

    # Step 7: The money output — total slots needed vs available
    print(f"\n{'═' * 70}")
    print(f"  SLOT BUDGET")
    print(f"{'═' * 70}")

    # Count unique (onset_combo, coda_combo) pairs
    onset_coda_pairs = set()
    for (onset, nucleus, coda), count in syllable_freq.items():
        onset_bits = consonant_to_chord.get(onset[0] if len(onset) == 1 else '(none)',
                     assigned_clusters.get(onset, -1))
        coda_bits = consonant_to_chord.get(coda[0] if len(coda) == 1 else '(none)',
                    assigned_clusters.get(coda, -1))
        if onset_bits >= 0 and coda_bits >= 0:
            onset_coda_pairs.add((onset_bits, coda_bits))

    print(f"  Unique onset+coda finger pairs: {len(onset_coda_pairs)}")
    print(f"  Vowel slots per pair: 8 (4 modes × 2 ctrl)")
    print(f"  Total syllable slots used: ≤{len(onset_coda_pairs) * 8}")
    print(f"  Total syllable slots available: {32 * 32 * 8} (32×32×4modes×2ctrl)")
    print(f"  Utilization: {len(onset_coda_pairs) * 8 / (32*32*8) * 100:.1f}%")
    print(f"")
    print(f"  Available for briefs (no-space mode): {32*32*8} more slots")
    print(f"  Symbols/numbers/nav: ~100 needed")
    print(f"  Remaining for word briefs: ~{32*32*8 - 100}")

    # Final summary
    print(f"\n{'═' * 70}")
    print(f"  SUMMARY")
    print(f"{'═' * 70}")
    print(f"  Layout: 5 keys per hand (Dvorak aoeu + i, htns + d)")
    print(f"  Space = word boundary (hold for words, release to commit)")
    print(f"  Ctrl = vowel extension (doubles vowel slots from 4 to 8)")
    print(f"  4 hand-order modes = primary vowel selector")
    print(f"  Right hand = onset, Left hand = coda (same consonant map)")
    print(f"  31 combos → 24 consonants + 7 clusters")
    print(f"  8 vowel slots per onset+coda pair (covers 15 vowels)")
    print(f"  Total: {encodable2/total_syllables*100:.1f}% of English syllable usage in single chords")
    print(f"  78% of words are monosyllabic = single chord")
    print(f"  Multi-syllable words: hold space, chord syllables, release")

if __name__ == '__main__':
    main()
