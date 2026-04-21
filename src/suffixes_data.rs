/// Suffix briefs — left-hand-only chords (right=0, no word key).
/// These backspace the trailing space from the previous word, append the suffix, then re-add space.
/// Left bits: I=0001 M=0010 R=0100 P=1000
pub const SUFFIXES: &[(u8, &str)] = &[
    (0b0001, "s"),      // plural / 3rd person — I (index)
    (0b0010, "ing"),    // progressive — M (middle)
    (0b0100, "ed"),     // past tense — R (ring)
    (0b1000, "'s"),     // possessive / contractions — P (pinky)
    (0b0011, "ly"),     // adverb — I+M
    (0b0101, "er"),     // comparative / agent — I+R
    (0b0110, "tion"),   // nominalization — M+R
    (0b1010, "ment"),   // nominalization — M+P
    (0b1100, "ness"),   // nominalization — R+P
    (0b0111, "able"),   // adjective — I+M+R
    (0b1001, "ity"),    // nominalization — I+P
    (0b1011, "ous"),    // adjective — I+M+P
    (0b1101, "ive"),    // adjective — I+R+P
    (0b1110, "al"),     // adjective — M+R+P
    (0b1111, "ful"),    // adjective — I+M+R+P (all four)
];
