/// Suffix briefs — left-hand-only chords (right=0, no word key).
/// Backspaces trailing space from previous word, appends suffix, re-adds space.
///
/// Ordered by measured finger effort (fastest → slowest).
/// Left bits: I=0001 M=0010 R=0100 P=1000
pub const SUFFIXES: &[(u8, &str)] = &[
    (0b0001, "s"),      //  668ms — index             plural / 3rd person
    (0b0100, "ed"),     //  703ms — ring              past tense
    (0b1000, "ing"),    //  721ms — pinky             progressive
    (0b0010, "ly"),     //  739ms — middle            adverb
    (0b1111, "'s"),     //  784ms — all four          possessive / contraction
    (0b0110, "er"),     //  754ms — middle+ring       comparative / agent
    (0b0011, "tion"),   //  843ms — index+middle      nominalization
    (0b0111, "al"),     //  809ms — index+middle+ring adjective
    (0b1001, "ment"),   //  895ms — index+pinky       nominalization
    (0b0101, "ness"),   //  913ms — index+ring        nominalization
    (0b1100, "able"),   //  950ms — ring+pinky        adjective
    (0b1110, "ive"),    //  992ms — middle+ring+pinky  adjective
    (0b1010, "ful"),    // 1099ms — middle+pinky      adjective
    (0b1101, "ous"),    // 1254ms — index+ring+pinky  adjective
    (0b1011, "ity"),    // 1516ms — index+middle+pinky nominalization
];
