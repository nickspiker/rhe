/// Auto-generated brief assignments. Edit and recompile to customize.
/// Format: (left_4bits, right_5bits, "word")
///
/// Bit encoding (both hands): index=bit0 (LSB), outward from center.
/// Left:  I=0001 M=0010 R=0100 P=1000
/// Right: I=0001 M=0010 R=0100 P=1000 T(thumb/mod)=10000
///
/// Note: binary literals read right-to-left (LSB first), so
/// 0b0110 = middle+ring, NOT ring+middle. Index is always the rightmost bit.
pub const BRIEFS: &[(u8, u8, &str)] = &[
    (0b0000, 0b00001, "you"),               // ergo #1 (natural: Y+UW)
    (0b0000, 0b00010, "i"),                 // ergo #2 (natural: -+AY)-
    (0b0000, 0b00100, "to"),                // ergo #3 (natural: T+UW)
    (0b0001, 0b00001, "a"),                 // ergo #4 (natural: -+AH)
    (0b0010, 0b00001, "it"),                // ergo #5 (natural: T+IH)
    (0b0001, 0b00010, "and"),               // ergo #6 (natural: N+AH)
    (0b0010, 0b00010, "that"),              // ergo #7 (natural: DH+AE)
    (0b0000, 0b10000, "the"),               // pinned
    (0b0000, 0b01000, "of"),                // ergo #8 (natural: V+AH)
    (0b0100, 0b00001, "is"),                // ergo #9 (natural: Z+IH)
    (0b0100, 0b00010, "in"),                // ergo #10 (natural: N+IH)
    (0b0000, 0b00011, "what"),              // ergo #11 (natural: W+AH)
    (0b0001, 0b00100, "we"),                // ergo #12 (natural: W+IY)
    (0b0010, 0b00100, "me"),                // ergo #13 (natural: M+IY)
    (0b1000, 0b00001, "this"),              // ergo #14 (natural: DH+IH)
    (0b1000, 0b00010, "he"),                // ergo #15 (natural: HH+IY)
    (0b0100, 0b00100, "for"),               // ergo #16 (natural: F+AO)
    (0b0000, 0b00110, "my"),                // ergo #17 (natural: M+AY)
    (0b0001, 0b01000, "on"),                // ergo #18 (natural: N+AA)
    (0b0010, 0b01000, "have"),              // ergo #19 (natural: HH+AE)
    (0b0001, 0b10000, "your"),              // ergo #20 (natural: Y+AO)
    (0b0010, 0b10000, "do"),                // ergo #21 (natural: D+UW)
    (0b0000, 0b10001, "no"),                // ergo #22 (natural: N+OW)
    (0b0000, 0b10010, "not"),               // ergo #23 (natural: N+AA)
    (0b0011, 0b00001, "be"),                // ergo #24 (natural: B+IY)
    (0b0011, 0b00010, "are"),               // ergo #25 (natural: R+AA)
    (0b0001, 0b00011, "know"),              // ergo #26 (natural: N+OW)
    (0b0010, 0b00011, "can"),               // ergo #27 (natural: K+AE)
    (0b1000, 0b00100, "with"),              // ergo #28 (natural: W+IH)
    (0b0000, 0b00101, "but"),               // ergo #29 (natural: B+AH)
    (0b0100, 0b01000, "all"),               // ergo #30 (natural: L+AO)
    (0b0100, 0b10000, "so"),                // ergo #31 (natural: S+OW)
    (0b0000, 0b10100, "just"),              // ergo #32 (natural: JH+AH)
    (0b0110, 0b00001, "there"),             // ergo #33 (natural: DH+EH)
    (0b0110, 0b00010, "here"),              // ergo #34 (natural: HH+IY)
    (0b0100, 0b00011, "they"),              // ergo #35 (natural: DH+EY)
    (0b0011, 0b00100, "like"),              // ergo #36 (natural: L+AY)
    (0b0001, 0b00110, "get"),               // ergo #37 (natural: G+EH)
    (0b0010, 0b00110, "she"),               // ergo #38 (natural: SH+IY)
    (0b0001, 0b10001, "go"),                // ergo #39 (natural: G+OW)
    (0b0010, 0b10001, "if"),                // ergo #40 (natural: F+IH)
    (0b0001, 0b10010, "right"),             // ergo #41 (natural: R+AY)
    (0b0010, 0b10010, "out"),               // ergo #42 (natural: T+AW)
    (0b1000, 0b01000, "about"),             // ergo #43 (natural: B+AH)
    (0b0000, 0b01010, "up"),                // ergo #44 (natural: P+AH)
    (0b0000, 0b01100, "at"),                // ergo #45 (natural: T+AE)
    (0b1000, 0b10000, "him"),               // ergo #46 (natural: HH+IH)
    (0b0000, 0b11000, "now"),               // ergo #47 (natural: N+AW)
    (0b0101, 0b00001, "oh"),                // ergo #48 (natural: -+OW)
    (0b0101, 0b00010, "one"),               // ergo #49 (natural: W+AH)
    (0b1000, 0b00011, "come"),              // ergo #50 (natural: K+AH)
    (0b0100, 0b00110, "let"),               // L+EH
    (0b0011, 0b00011, "an"),                // bumped from N+Ae → N+Iy
    (0b0011, 0b01000, "people"),            // P+IY
    (0b0010, 0b10100, "give"),              // G+IH
    (0b0000, 0b10011, "am"),                // bumped from M+Ae → M+-
    (0b0001, 0b10100, "god"),               // bumped from G+Aa → G+Ah
    (0b0001, 0b00101, "uh"),                // bumped from -+Ah → R+Ah
    (0b0110, 0b00100, "keep"),              // bumped from K+Iy → K+Ey
    (0b0010, 0b00101, "enough"),            // bumped from N+Ih → R+Ih
    (0b0100, 0b10001, "next"),              // bumped from N+Eh → D+Eh
    (0b0100, 0b10010, "care"),              // bumped from K+Eh → Z+Eh
    (0b0011, 0b10000, "seen"),              // bumped from S+Iy → -+mod+Iy
    (0b0010, 0b01010, "will"),              // W+IH
    (0b0010, 0b01100, "think"),             // TH+IH
    (0b1000, 0b10010, "as"),                // Z+AE
    (0b1010, 0b00010, "see"),               // bumped from S+Iy → S+Ow
    (0b0001, 0b01010, "when"),              // bumped from W+Eh → W+Ah
    (0b1100, 0b00010, "us"),                // bumped from S+Ah → S+Ao
    (0b1010, 0b00001, "take"),              // bumped from T+Ey → T+Ow
    (0b1100, 0b00001, "tell"),              // bumped from T+Eh → T+Ao
    (0b0110, 0b00011, "need"),              // bumped from N+Iy → N+Ey
    (0b0010, 0b11000, "because"),           // B+IH
    (0b1000, 0b00110, "little"),            // bumped from L+Ih → L+Ae
    (0b0110, 0b01000, "please"),            // bumped from P+Iy → P+Ey
    (0b0011, 0b00110, "love"),              // bumped from L+Ah → L+Iy
    (0b0001, 0b10011, "much"),              // M+AH
    (0b0100, 0b10100, "again"),             // bumped from G+Ah → G+Eh
    (0b0010, 0b10011, "maybe"),             // bumped from M+Ey → M+Ih
    (0b0001, 0b11000, "before"),            // bumped from B+Ih → B+Ah
    (0b0101, 0b00100, "stop"),              // bumped from S+Aa → K+Aa
    (0b0110, 0b10000, "name"),              // bumped from N+Ey → -+mod+Ey
    (0b0001, 0b01100, "someone"),           // bumped from S+Ah → Th+Ah
    (0b0000, 0b01001, "fine"),              // bumped from F+Ay → F+-
    (0b0000, 0b00111, "house"),             // bumped from H+Aw → H+-
    (0b1000, 0b10001, "dad"),               // D+AE
    (0b0100, 0b00101, "wrong"),             // bumped from R+Ao → R+Eh
    (0b0011, 0b10001, "dead"),              // bumped from D+Eh → D+Iy
    (0b0011, 0b10010, "meet"),              // bumped from M+Iy → Z+Iy
    (0b0000, 0b10110, "stuff"),             // bumped from S+Ah → L+mod+-
    (0b0100, 0b01010, "well"),              // W+EH
    (0b0001, 0b01001, "from"),              // F+AH
    (0b0010, 0b00111, "his"),               // HH+IH
    (0b1001, 0b00001, "time"),              // T+AY
    (0b1010, 0b00100, "okay"),              // K+OW
    (0b0000, 0b10101, "then"),              // bumped from Dh+Eh → Dh+-
    (0b1001, 0b00010, "some"),              // bumped from S+Ah → S+Ay
    (0b0111, 0b00010, "say"),               // bumped from S+Ey → S+Er
    (0b0101, 0b00011, "never"),             // bumped from N+Eh → N+Aa
    (0b0111, 0b00001, "two"),               // bumped from T+Uw → T+Er
    (0b0110, 0b10001, "day"),               // D+EY
    (0b0100, 0b10011, "must"),              // bumped from M+Ah → M+Eh
    (0b1100, 0b00100, "call"),              // K+AO
    (0b0110, 0b00110, "last"),              // bumped from L+Ae → L+Ey
    (0b0101, 0b01000, "place"),             // bumped from P+Ey → P+Aa
    (0b0100, 0b11000, "big"),               // bumped from B+Ih → B+Eh
    (0b0001, 0b00111, "hello"),             // HH+AH
    (0b0110, 0b10010, "stay"),              // bumped from S+Ey → Z+Ey
    (0b0001, 0b10110, "another"),           // bumped from N+Ah → L+mod+Ah
    (0b1000, 0b00101, "remember"),          // bumped from R+Ih → R+Ae
    (0b1000, 0b10100, "ask"),               // bumped from S+Ae → G+Ae
    (0b0101, 0b10000, "car"),               // bumped from K+Aa → -+mod+Aa
    (0b0010, 0b01001, "family"),            // bumped from F+Ae → F+Ih
    (0b0011, 0b00101, "real"),              // R+IY
    (0b0010, 0b10110, "miss"),              // bumped from M+Ih → L+mod+Ih
    (0b0011, 0b10100, "guess"),             // bumped from G+Eh → G+Iy
    (0b0100, 0b01100, "end"),               // bumped from N+Eh → Th+Eh
    (0b1000, 0b11000, "back"),              // B+AE
    (0b1000, 0b01010, "where"),             // bumped from W+Eh → W+Ae
    (0b1000, 0b10011, "man"),               // M+AE
    (0b1010, 0b00011, "only"),              // N+OW
    (0b0011, 0b10011, "mean"),              // M+IY
    (0b1000, 0b01100, "thank"),             // TH+AE
    (0b1100, 0b00011, "any"),               // bumped from N+Eh → N+Ao
    (0b0000, 0b01011, "sure"),              // bumped from Sh+Uh → Sh+-
    (0b0100, 0b00111, "help"),              // HH+EH
    (0b0001, 0b10101, "their"),             // bumped from Dh+Eh → Dh+Ah
    (0b0010, 0b10101, "other"),             // bumped from Dh+Ah → Dh+Ih
    (0b0011, 0b01010, "wait"),              // bumped from W+Ey → W+Iy
    (0b0110, 0b10100, "great"),             // G+EY
    (0b0101, 0b00110, "long"),              // bumped from L+Ao → L+Aa
    (0b1001, 0b00100, "nice"),              // bumped from N+Ay → K+Ay
    (0b0011, 0b11000, "believe"),           // bumped from B+Ih → B+Iy
    (0b0111, 0b00100, "kind"),              // bumped from K+Ay → K+Er
    (0b0011, 0b01100, "three"),             // TH+IY
    (0b1010, 0b01000, "own"),               // bumped from N+Ow → P+Ow
    (0b0110, 0b00101, "same"),              // bumped from S+Ey → R+Ey
    (0b0100, 0b01001, "friend"),            // F+EH
    (0b1100, 0b01000, "saw"),               // bumped from S+Ao → P+Ao
    (0b1100, 0b10000, "already"),           // bumped from L+Ao → -+mod+Ao
    (0b1010, 0b10000, "most"),              // bumped from M+Ow → -+mod+Ow
    (0b0101, 0b10001, "start"),             // bumped from S+Aa → D+Aa
    (0b0101, 0b10010, "wanna"),             // bumped from W+Aa → Z+Aa
    (0b0100, 0b10110, "anyone"),            // bumped from N+Eh → L+mod+Eh
    (0b0000, 0b01110, "under"),             // bumped from N+Ah → Ng+-
    (0b0000, 0b11010, "whatever"),          // bumped from W+Ah → W+mod+-
    (0b0000, 0b11100, "inside"),            // bumped from N+Ih → Th+mod+-
    (0b0100, 0b10101, "them"),              // DH+EH
    (0b0101, 0b10100, "gonna"),             // G+AA
    (0b1011, 0b00010, "something"),         // bumped from S+Ah → S+Uw
    (0b1011, 0b00001, "too"),               // T+UW
    (0b0110, 0b01010, "way"),               // W+EY
    (0b0110, 0b10011, "make"),              // M+EY
    (0b1110, 0b00010, "sorry"),             // bumped from S+Aa → S+Uh
    (0b1001, 0b00011, "into"),              // bumped from N+Ih → N+Ay
    (0b0111, 0b00011, "anything"),          // bumped from N+Eh → N+Er
    (0b1000, 0b01001, "after"),             // F+AE
    (0b1110, 0b00001, "talk"),              // bumped from T+Ao → T+Uh
    (0b0000, 0b11001, "everything"),        // bumped from V+Eh → V+-
    (0b1100, 0b00110, "always"),            // L+AO
    (0b1010, 0b00110, "leave"),             // bumped from L+Iy → L+Ow
    (0b0011, 0b01001, "feel"),              // F+IY
    (0b0110, 0b11000, "bad"),               // bumped from B+Ae → B+Ey
    (0b0001, 0b01011, "understand"),        // bumped from N+Ah → Sh+Ah
    (0b0011, 0b00111, "hear"),              // HH+IY
    (0b0001, 0b01110, "son"),               // bumped from S+Ah → Ng+Ah
    (0b1001, 0b01000, "try"),               // bumped from T+Ay → P+Ay
    (0b1000, 0b00111, "hell"),              // bumped from H+Eh → H+Ae
    (0b0001, 0b11010, "together"),          // bumped from T+Ah → W+mod+Ah
    (0b1001, 0b10000, "live"),              // bumped from L+Ay → -+mod+Ay
    (0b1000, 0b10110, "actually"),          // bumped from K+Ae → L+mod+Ae
    (0b0010, 0b01011, "shit"),              // SH+IH
    (0b0001, 0b11100, "once"),              // bumped from W+Ah → Th+mod+Ah
    (0b0101, 0b00101, "ready"),             // bumped from R+Eh → R+Aa
    (0b1100, 0b10001, "door"),              // D+AO
    (0b1100, 0b10010, "also"),              // bumped from L+Ao → Z+Ao
    (0b0111, 0b01000, "pretty"),            // bumped from P+Ih → P+Er
    (0b0110, 0b01100, "haven"),             // bumped from H+Ey → Th+Ey
    (0b1010, 0b10001, "whole"),             // bumped from H+Ow → D+Ow
    (0b0010, 0b01110, "since"),             // bumped from S+Ih → Ng+Ih
    (0b1010, 0b10010, "hope"),              // bumped from H+Ow → Z+Ow
    (0b0010, 0b11010, "excuse"),            // bumped from K+Ih → W+mod+Ih
    (0b0111, 0b10000, "turn"),              // bumped from T+Er → -+mod+Er
    (0b0010, 0b11100, "sit"),               // bumped from S+Ih → Th+mod+Ih
    (0b0011, 0b10110, "eat"),               // bumped from T+Iy → L+mod+Iy
    (0b0000, 0b01101, "somebody"),          // bumped from S+Ah → Ch+-
    (0b0000, 0b10111, "afraid"),            // bumped from F+Ah → H+mod+-
    (0b0101, 0b01010, "want"),              // W+AA
    (0b1100, 0b00101, "or"),                // R+AO
    (0b1101, 0b00001, "our"),               // bumped from -+Aw → T+Aw
    (0b0110, 0b00111, "hey"),               // HH+EY
    (0b1110, 0b00100, "could"),             // K+UH
    (0b0011, 0b10101, "these"),             // DH+IY
    (0b1101, 0b00010, "still"),             // bumped from S+Ih → S+Aw
    (0b1001, 0b00110, "life"),              // L+AY
    (0b1000, 0b10101, "than"),              // DH+AE
    (0b0101, 0b10011, "money"),             // bumped from M+Ah → M+Aa
    (0b0001, 0b11001, "ever"),              // bumped from V+Eh → V+Ah
    (0b0111, 0b00110, "old"),               // bumped from L+Ow → L+Er
    (0b0010, 0b11001, "every"),             // bumped from V+Eh → V+Ih
    (0b1011, 0b00100, "ok"),                // bumped from K+Ow → K+Uw
    (0b0101, 0b11000, "baby"),              // bumped from B+Ey → B+Aa
    (0b1001, 0b10001, "idea"),              // D+AY
    (0b0001, 0b01101, "such"),              // bumped from S+Ah → Ch+Ah
    (0b0110, 0b01001, "fuck"),              // bumped from F+Ah → F+Ey
    (0b1001, 0b10010, "while"),             // bumped from W+Ay → Z+Ay
    (0b0001, 0b10111, "tomorrow"),          // bumped from T+Ah → H+mod+Ah
    (0b1010, 0b00101, "run"),               // bumped from R+Ah → R+Ow
    (0b0101, 0b01100, "hard"),              // bumped from H+Aa → Th+Aa
    (0b1010, 0b10100, "gotta"),             // bumped from G+Aa → G+Ow
    (0b1100, 0b10100, "ago"),               // bumped from G+Ah → G+Ao
    (0b0110, 0b10110, "case"),              // bumped from K+Ey → L+mod+Ey
    (0b0111, 0b10001, "die"),               // bumped from D+Ay → D+Er
    (0b0111, 0b10010, "worry"),             // bumped from W+Er → Z+Er
    (0b0100, 0b01011, "second"),            // bumped from S+Eh → Sh+Eh
    (0b0010, 0b01101, "minute"),            // bumped from M+Ih → Ch+Ih
    (0b0010, 0b10111, "kid"),               // bumped from K+Ih → H+mod+Ih
    (0b0100, 0b01110, "dear"),              // bumped from D+Ih → Ng+Eh
    (0b0100, 0b11010, "anyway"),            // bumped from N+Eh → W+mod+Eh
    (0b0100, 0b11100, "fun"),               // bumped from F+Ah → Th+mod+Eh
    (0b1100, 0b10011, "more"),              // M+AO
    (0b0100, 0b11001, "very"),              // V+EH
    (0b1011, 0b00011, "nothing"),           // bumped from N+Ah → N+Uw
    (0b1010, 0b01010, "away"),              // bumped from W+Ah → W+Ow
    (0b1110, 0b00011, "night"),             // bumped from N+Ay → N+Uh
    (0b0101, 0b01001, "father"),            // F+AA
    (0b1001, 0b10100, "guy"),               // G+AY
    (0b1100, 0b01010, "which"),             // bumped from W+Ih → W+Ao
    (0b0111, 0b10100, "girl"),              // G+ER
    (0b1101, 0b00100, "course"),            // bumped from K+Ao → K+Aw
    (0b1010, 0b10011, "may"),               // bumped from M+Ey → M+Ow
    (0b1011, 0b01000, "move"),              // bumped from M+Uw → P+Uw
    (0b0101, 0b00111, "huh"),               // bumped from H+Ah → H+Aa
    (0b1010, 0b11000, "both"),              // B+OW
    (0b1000, 0b01011, "happy"),             // bumped from H+Ae → Sh+Ae
    (0b1100, 0b11000, "brother"),           // bumped from B+Ah → B+Ao
    (0b1001, 0b00101, "wife"),              // bumped from W+Ay → R+Ay
    (0b1000, 0b01110, "matter"),            // bumped from M+Ae → Ng+Ae
    (0b0101, 0b10110, "ah"),                // bumped from -+Aa → L+mod+Aa
    (0b1011, 0b10000, "school"),            // bumped from S+Uw → -+mod+Uw
    (0b1110, 0b01000, "play"),              // bumped from P+Ey → P+Uh
    (0b1000, 0b11010, "hand"),              // bumped from H+Ae → W+mod+Ae
    (0b1100, 0b01100, "water"),             // bumped from W+Ao → Th+Ao
    (0b0111, 0b00101, "person"),            // bumped from P+Er → R+Er
    (0b0110, 0b10101, "late"),              // bumped from L+Ey → Dh+Ey
    (0b1000, 0b11100, "happen"),            // bumped from H+Ae → Th+mod+Ae
    (0b0011, 0b01011, "shut"),              // bumped from Sh+Ah → Sh+Iy
    (0b0100, 0b01101, "check"),             // CH+EH
    (0b0011, 0b01110, "deal"),              // bumped from D+Iy → Ng+Iy
    (0b0011, 0b11010, "sleep"),             // bumped from S+Iy → W+mod+Iy
    (0b1010, 0b01100, "close"),             // bumped from K+Ow → Th+Ow
    (0b1110, 0b10000, "important"),         // bumped from M+Ih → -+mod+Uh
    (0b0100, 0b10111, "set"),               // bumped from S+Eh → H+mod+Eh
    (0b0000, 0b11011, "number"),            // bumped from N+Ah → Zh+-
    (0b0011, 0b11100, "least"),             // bumped from L+Iy → Th+mod+Iy
    (0b0000, 0b11110, "wish"),              // bumped from W+Ih → Ng+mod+-
    (0b1001, 0b01010, "why"),               // W+AY
    (0b1110, 0b00110, "look"),              // L+UH
    (0b1001, 0b11000, "by"),                // B+AY
    (0b1100, 0b01001, "off"),               // F+AO
    (0b0011, 0b11001, "even"),              // V+IY
    (0b0111, 0b01010, "work"),              // W+ER
    (0b1010, 0b00111, "home"),              // HH+OW
    (0b1101, 0b00011, "new"),               // bumped from N+Uw → N+Aw
    (0b1011, 0b00110, "lot"),               // bumped from L+Aa → L+Uw
    (0b1001, 0b10011, "mother"),            // bumped from M+Ah → M+Ay
    (0b0111, 0b10011, "might"),             // bumped from M+Ay → M+Er
    (0b1100, 0b00111, "heard"),             // bumped from H+Er → H+Ao
    (0b0111, 0b11000, "bit"),               // bumped from B+Ih → B+Er
    (0b0000, 0b01111, "yet"),               // bumped from Y+Eh → Y+-
    (0b0001, 0b11011, "tonight"),           // bumped from T+Ah → Zh+Ah
    (0b1000, 0b11001, "everyone"),          // bumped from V+Eh → V+Ae
    (0b0001, 0b11110, "alone"),             // bumped from L+Ah → Ng+mod+Ah
    (0b1001, 0b01100, "myself"),            // bumped from M+Ay → Th+Ay
    (0b1010, 0b01001, "phone"),             // F+OW
    (0b1101, 0b01000, "problem"),           // bumped from P+Aa → P+Aw
    (0b1011, 0b10001, "true"),              // bumped from T+Uw → D+Uw
    (0b0101, 0b10101, "heart"),             // bumped from H+Aa → Dh+Aa
    (0b1011, 0b10010, "soon"),              // bumped from S+Uw → Z+Uw
    (0b0011, 0b01101, "each"),              // CH+IY
    (0b1110, 0b10001, "doctor"),            // bumped from D+Aa → D+Uh
    (0b0110, 0b01011, "pay"),               // bumped from P+Ey → Sh+Ey
    (0b0110, 0b01110, "crazy"),             // bumped from K+Ey → Ng+Ey
    (0b1000, 0b01101, "damn"),              // bumped from D+Ae → Ch+Ae
    (0b0010, 0b11011, "drink"),             // bumped from D+Ih → Zh+Ih
    (0b0010, 0b11110, "its"),               // bumped from T+Ih → Ng+mod+Ih
    (0b1110, 0b10010, "easy"),              // bumped from Z+Iy → Z+Uh
    (0b1100, 0b10110, "four"),              // bumped from F+Ao → L+mod+Ao
    (0b0111, 0b01100, "word"),              // bumped from W+Er → Th+Er
    (0b1010, 0b10110, "moment"),            // bumped from M+Ow → L+mod+Ow
    (0b0011, 0b10111, "week"),              // bumped from W+Iy → H+mod+Iy
    (0b1101, 0b10000, "husband"),           // bumped from H+Ah → -+mod+Aw
    (0b0110, 0b11010, "game"),              // bumped from G+Ey → W+mod+Ey
    (0b0000, 0b11101, "mr"),                // bumped from M+Ih → Jh+-
    (0b1000, 0b10111, "stand"),             // bumped from S+Ae → H+mod+Ae
    (0b0110, 0b11100, "cut"),               // bumped from K+Ah → Th+mod+Ey
    (0b0111, 0b00111, "her"),               // HH+ER
    (0b1110, 0b10100, "good"),              // G+UH
    (0b1101, 0b10001, "down"),              // D+AW
    (0b0111, 0b01001, "first"),             // F+ER
    (0b1001, 0b01001, "find"),              // F+AY
    (0b1111, 0b00010, "sir"),               // bumped from S+Er → S+Oy
    (0b1010, 0b10101, "those"),             // DH+OW
    (0b1111, 0b00001, "today"),             // bumped from T+Ah → T+Oy
    (0b1101, 0b00110, "listen"),            // bumped from L+Ih → L+Aw
    (0b1001, 0b00111, "hi"),                // HH+AY
    (0b1011, 0b00101, "room"),              // R+UW
    (0b0001, 0b01111, "um"),                // bumped from M+Ah → Y+Ah
    (0b0001, 0b11101, "until"),             // bumped from N+Ah → Jh+Ah
    (0b0010, 0b01111, "year"),              // Y+IH
    (0b0101, 0b01011, "watch"),             // bumped from W+Aa → Sh+Aa
    (0b1011, 0b10100, "music"),             // bumped from M+Uw → G+Uw
    (0b0101, 0b01110, "probably"),          // bumped from P+Aa → Ng+Aa
    (0b0101, 0b11010, "part"),              // bumped from P+Aa → W+mod+Aa
    (0b0110, 0b11001, "everybody"),         // bumped from V+Eh → V+Ey
    (0b0110, 0b01101, "change"),            // CH+EY
    (0b1001, 0b10110, "five"),              // bumped from F+Ay → L+mod+Ay
    (0b0101, 0b11100, "far"),               // bumped from F+Aa → Th+mod+Aa
    (0b0010, 0b11101, "hit"),               // bumped from H+Ih → Jh+Ih
    (0b1100, 0b10101, "story"),             // bumped from S+Ao → Dh+Ao
    (0b0111, 0b10110, "hurt"),              // bumped from H+Er → L+mod+Er
    (0b1110, 0b00101, "rest"),              // bumped from R+Eh → R+Uh
    (0b1101, 0b10010, "sister"),            // bumped from S+Ih → Z+Aw
    (0b0100, 0b11011, "between"),           // bumped from B+Ih → Zh+Eh
    (0b0100, 0b11110, "behind"),            // bumped from B+Ih → Ng+mod+Eh
    (0b0110, 0b10111, "blood"),             // bumped from B+Ah → H+mod+Ey
    (0b0100, 0b01111, "yes"),               // Y+EH
    (0b1110, 0b01010, "would"),             // W+UH
    (0b1011, 0b01100, "through"),           // TH+UW
    (0b1011, 0b01010, "world"),             // bumped from W+Er → W+Uw
    (0b1010, 0b01011, "show"),              // SH+OW
    (0b1111, 0b00100, "kill"),              // bumped from K+Ih → K+Oy
    (0b1011, 0b10011, "many"),              // bumped from M+Eh → M+Uw
    (0b1110, 0b10011, "mind"),              // bumped from M+Ay → M+Uh
    (0b1011, 0b11000, "bring"),             // bumped from B+Ih → B+Uw
    (0b1110, 0b11000, "beautiful"),         // bumped from B+Uw → B+Uh
    (0b1101, 0b10100, "exactly"),           // bumped from G+Ih → G+Aw
    (0b1001, 0b10101, "mine"),              // bumped from M+Ay → Dh+Ay
    (0b0101, 0b01101, "chance"),            // bumped from Ch+Ae → Ch+Aa
    (0b0101, 0b11001, "party"),             // bumped from P+Aa → V+Aa
    (0b1100, 0b01011, "shall"),             // bumped from Sh+Ae → Sh+Ao
    (0b1100, 0b01110, "daughter"),          // bumped from D+Ao → Ng+Ao
    (0b1010, 0b01110, "nobody"),            // bumped from N+Ow → Ng+Ow
    (0b1101, 0b00101, "read"),              // bumped from R+Eh → R+Aw
    (0b0111, 0b10101, "though"),            // bumped from Dh+Ow → Dh+Er
    (0b0011, 0b11011, "speak"),             // bumped from S+Iy → Zh+Iy
    (0b1100, 0b11010, "almost"),            // bumped from L+Ao → W+mod+Ao
    (0b0100, 0b11101, "anymore"),           // bumped from N+Eh → Jh+Eh
    (0b1110, 0b01100, "trouble"),           // bumped from T+Ah → Th+Uh
    (0b1100, 0b11100, "war"),               // bumped from W+Ao → Th+mod+Ao
    (0b1010, 0b11010, "city"),              // bumped from S+Ih → W+mod+Ow
    (0b1000, 0b11011, "trust"),             // bumped from T+Ah → Zh+Ae
    (0b1010, 0b11100, "question"),          // bumped from K+Eh → Th+mod+Ow
    (0b1000, 0b11110, "welcome"),           // bumped from W+Eh → Ng+mod+Ae
    (0b0101, 0b10111, "couple"),            // bumped from K+Ah → H+mod+Aa
    (0b0011, 0b11110, "free"),              // bumped from F+Iy → Ng+mod+Iy
    (0b1000, 0b01111, "yeah"),              // Y+AE
    (0b1011, 0b00111, "who"),               // HH+UW
    (0b1010, 0b11001, "over"),              // V+OW
    (0b1111, 0b00011, "around"),            // bumped from N+Er → N+Oy
    (0b1101, 0b01010, "woman"),             // bumped from W+Uh → W+Aw
    (0b1101, 0b10011, "mom"),               // bumped from M+Aa → M+Aw
    (0b1110, 0b00111, "head"),              // bumped from H+Eh → H+Uh
    (0b1011, 0b01001, "few"),               // F+UW
    (0b1111, 0b01000, "open"),              // bumped from P+Ow → P+Oy
    (0b1110, 0b01001, "face"),              // bumped from F+Ey → F+Uh
    (0b0011, 0b01111, "young"),             // bumped from Y+Ah → Y+Iy
    (0b1111, 0b10000, "point"),             // bumped from P+Oy → -+mod+Oy
    (0b1101, 0b11000, "body"),              // bumped from B+Aa → B+Aw
    (0b1001, 0b01011, "quite"),             // bumped from K+Ay → Sh+Ay
    (0b1001, 0b01110, "fight"),             // bumped from F+Ay → Ng+Ay
    (0b1001, 0b11010, "fire"),              // bumped from F+Ay → W+mod+Ay
    (0b1001, 0b11100, "side"),              // bumped from S+Ay → Th+mod+Ay
    (0b1011, 0b10110, "truth"),             // bumped from T+Uw → L+mod+Uw
    (0b0110, 0b11011, "able"),              // bumped from B+Ey → Zh+Ey
    (0b0110, 0b11110, "lady"),              // bumped from L+Ey → Ng+mod+Ey
    (0b0111, 0b01011, "shot"),              // bumped from Sh+Aa → Sh+Er
    (0b1100, 0b01101, "walk"),              // bumped from W+Ao → Ch+Ao
    (0b1101, 0b01100, "town"),              // bumped from T+Aw → Th+Aw
    (0b1100, 0b11001, "office"),            // bumped from F+Ao → V+Ao
    (0b1000, 0b11101, "half"),              // bumped from H+Ae → Jh+Ae
    (0b1010, 0b01101, "whoa"),              // bumped from W+Ow → Ch+Ow
    (0b0000, 0b11111, "honey"),             // bumped from H+Ah → Y+mod+-
    (0b0111, 0b01110, "front"),             // bumped from F+Ah → Ng+Er
    (0b0011, 0b11101, "team"),              // bumped from T+Iy → Jh+Iy
    (0b1110, 0b10110, "gun"),               // bumped from G+Ah → L+mod+Uh
    (0b1010, 0b10111, "send"),              // bumped from S+Eh → H+mod+Ow
    (0b1100, 0b10111, "bed"),               // bumped from B+Eh → H+mod+Ao
    (0b0111, 0b11010, "hurry"),             // bumped from H+Er → W+mod+Er
    (0b0111, 0b11100, "sometimes"),         // bumped from S+Ah → Th+mod+Er
    (0b1101, 0b00111, "how"),               // HH+AW
    (0b1111, 0b00110, "else"),              // bumped from L+Eh → L+Oy
    (0b1101, 0b01001, "found"),             // F+AW
    (0b0001, 0b11111, "police"),            // bumped from P+Ah → Y+mod+Ah
    (0b1111, 0b10001, "death"),             // bumped from D+Eh → D+Oy
    (0b0010, 0b11111, "different"),         // bumped from D+Ih → Y+mod+Ih
    (0b1001, 0b01101, "child"),             // CH+AY
    (0b0110, 0b01111, "break"),             // bumped from B+Ey → Y+Ey
    (0b1001, 0b11001, "high"),              // bumped from H+Ay → V+Ay
    (0b1101, 0b10110, "wow"),               // bumped from W+Aw → L+mod+Aw
    (0b1011, 0b10101, "cool"),              // bumped from K+Uw → Dh+Uw
    (0b1110, 0b10101, "either"),            // bumped from Dh+Iy → Dh+Uh
    (0b1001, 0b10111, "bye"),               // bumped from B+Ay → H+mod+Ay
    (0b0110, 0b11101, "save"),              // bumped from S+Ey → Jh+Ey
    (0b0111, 0b01101, "become"),            // bumped from B+Ih → Ch+Er
    (0b1111, 0b10010, "along"),             // bumped from L+Ah → Z+Oy
    (0b0111, 0b11001, "l"),                 // bumped from L+Eh → V+Er
    (0b0101, 0b11011, "country"),           // bumped from K+Ah → Zh+Aa
    (0b0101, 0b11110, "clear"),             // bumped from K+Ih → Ng+mod+Aa
    (0b0111, 0b10111, "em"),                // bumped from M+Eh → H+mod+Er
    (0b1110, 0b01011, "should"),            // SH+UH
    (0b0101, 0b11101, "job"),               // JH+AA
    (0b1111, 0b10100, "against"),           // bumped from G+Ah → G+Oy
    (0b1111, 0b00101, "reason"),            // bumped from R+Iy → R+Oy
    (0b0100, 0b11111, "met"),               // bumped from M+Eh → Y+mod+Eh
    (0b1101, 0b10101, "power"),             // bumped from P+Aw → Dh+Aw
    (0b1011, 0b01011, "stupid"),            // bumped from S+Uw → Sh+Uw
    (0b1110, 0b01110, "full"),              // bumped from F+Uh → Ng+Uh
    (0b1011, 0b01110, "food"),              // bumped from F+Uw → Ng+Uw
    (0b1100, 0b11011, "dog"),               // bumped from D+Ao → Zh+Ao
    (0b1100, 0b11110, "order"),             // bumped from R+Ao → Ng+mod+Ao
    (0b0101, 0b01111, "fact"),              // bumped from F+Ae → Y+Aa
    (0b1011, 0b11010, "captain"),           // bumped from K+Ae → W+mod+Uw
    (0b1110, 0b11010, "six"),               // bumped from S+Ih → W+mod+Uh
    (0b1010, 0b11011, "funny"),             // bumped from F+Ah → Zh+Ow
    (0b1011, 0b11100, "black"),             // bumped from B+Ae → Th+mod+Uw
    (0b1110, 0b11100, "alive"),             // bumped from L+Ah → Th+mod+Uh
    (0b1010, 0b11110, "pick"),              // bumped from P+Ih → Ng+mod+Ow
    (0b1111, 0b10011, "morning"),           // bumped from M+Ao → M+Oy
    (0b1111, 0b11000, "boy"),               // B+OY
    (0b1111, 0b01010, "without"),           // bumped from W+Ih → W+Oy
    (0b1001, 0b11011, "buy"),               // bumped from B+Ay → Zh+Ay
    (0b1000, 0b11111, "answer"),            // bumped from N+Ae → Y+mod+Ae
    (0b1001, 0b11110, "line"),              // bumped from L+Ay → Ng+mod+Ay
    (0b1101, 0b01011, "outside"),           // bumped from T+Aw → Sh+Aw
    (0b1100, 0b01111, "lord"),              // bumped from L+Ao → Y+Ao
    (0b1111, 0b01100, "cause"),             // bumped from K+Aa → Th+Oy
    (0b1011, 0b01101, "ahead"),             // bumped from H+Ah → Ch+Uw
    (0b1011, 0b11001, "lose"),              // bumped from L+Uw → V+Uw
    (0b1110, 0b01101, "king"),              // bumped from K+Ih → Ch+Uh
    (0b1101, 0b01110, "plan"),              // bumped from P+Ae → Ng+Aw
    (0b1010, 0b01111, "dinner"),            // bumped from D+Ih → Y+Ow
    (0b1100, 0b11101, "sort"),              // bumped from S+Ao → Jh+Ao
    (0b1110, 0b11001, "boss"),              // bumped from B+Aa → V+Uh
    (0b1101, 0b11010, "promise"),           // bumped from P+Aa → W+mod+Aw
    (0b1101, 0b11100, "safe"),              // bumped from S+Ey → Th+mod+Aw
    (0b1010, 0b11101, "ma"),                // bumped from M+Aa → Jh+Ow
    (0b1110, 0b10111, "book"),              // bumped from B+Uh → H+mod+Uh
    (0b1011, 0b10111, "sent"),              // bumped from S+Eh → H+mod+Uw
    (0b0111, 0b11011, "anybody"),           // bumped from N+Eh → Zh+Er
    (0b0111, 0b11110, "small"),             // bumped from S+Ao → Ng+mod+Er
    (0b0011, 0b11111, "special"),           // bumped from S+Eh → Y+mod+Iy
    (0b0111, 0b01111, "yourself"),          // Y+ER
    (0b1111, 0b00111, "hold"),              // bumped from H+Ow → H+Oy
    (0b1111, 0b01001, "forget"),            // bumped from F+Er → F+Oy
    (0b0110, 0b11111, "hate"),              // bumped from H+Ey → Y+mod+Ey
    (0b1001, 0b01111, "light"),             // bumped from L+Ay → Y+Ay
    (0b1001, 0b11101, "sighs"),             // bumped from S+Ay → Jh+Ay
    (0b1101, 0b01101, "hour"),              // bumped from -+Aw → Ch+Aw
    (0b0111, 0b11101, "perfect"),           // bumped from P+Er → Jh+Er
    (0b1101, 0b11001, "parents"),           // bumped from P+Eh → V+Aw
    (0b1111, 0b10110, "s"),                 // bumped from S+Eh → L+mod+Oy
    (0b1101, 0b10111, "himself"),           // bumped from H+Ih → H+mod+Aw
    (0b0101, 0b11111, "hot"),               // bumped from H+Aa → Y+mod+Aa
    (0b1111, 0b10101, "serious"),           // bumped from S+Ih → Dh+Oy
    (0b1011, 0b11011, "sick"),              // bumped from S+Ih → Zh+Uw
    (0b1110, 0b11011, "company"),           // bumped from K+Ah → Zh+Uh
    (0b1011, 0b11110, "ha"),                // bumped from H+Aa → Ng+mod+Uw
    (0b1110, 0b11110, "scared"),            // bumped from S+Eh → Ng+mod+Uh
    (0b1011, 0b01111, "use"),               // Y+UW
    (0b1100, 0b11111, "alright"),           // bumped from L+Ao → Y+mod+Ao
    (0b1011, 0b11101, "john"),              // bumped from Jh+Aa → Jh+Uw
    (0b1111, 0b01110, "uncle"),             // bumped from Ng+Ah → Ng+Oy
    (0b1111, 0b01011, "red"),               // bumped from R+Eh → Sh+Oy
    (0b1110, 0b01111, "past"),              // bumped from P+Ae → Y+Uh
    (0b1111, 0b11010, "earth"),             // bumped from Th+Er → W+mod+Oy
    (0b1101, 0b11011, "possible"),          // bumped from P+Aa → Zh+Aw
    (0b1111, 0b11100, "shoot"),             // bumped from Sh+Uw → Th+mod+Oy
    (0b1110, 0b11101, "touch"),             // bumped from T+Ah → Jh+Uh
    (0b1101, 0b11110, "sound"),             // bumped from S+Aw → Ng+mod+Aw
    (0b1010, 0b11111, "top"),               // bumped from T+Aa → Y+mod+Ow
    (0b1001, 0b11111, "white"),             // bumped from W+Ay → Y+mod+Ay
    (0b0111, 0b11111, "perhaps"),           // bumped from P+Er → Y+mod+Er
    (0b1111, 0b01101, "ass"),               // bumped from S+Ae → Ch+Oy
    (0b1101, 0b01111, "cannot"),            // bumped from K+Ae → Y+Aw
    (0b1111, 0b11001, "win"),               // bumped from W+Ih → V+Oy
    (0b1101, 0b11101, "glad"),              // bumped from G+Ae → Jh+Aw
    (0b1111, 0b10111, "control"),           // bumped from K+Ah → H+mod+Oy
    (0b1011, 0b11111, "poor"),              // bumped from P+Uw → Y+mod+Uw
    (0b1111, 0b11011, "hmm"),               // bumped from H+- → Zh+Oy
    (0b1111, 0b11110, "human"),             // bumped from H+Uw → Ng+mod+Oy
    (0b1110, 0b11111, "drive"),             // bumped from D+Ay → Y+mod+Uh
    (0b1111, 0b01111, "hair"),              // bumped from H+Eh → Y+Oy
    (0b1111, 0b11101, "jack"),              // bumped from Jh+Ae → Jh+Oy
    (0b1101, 0b11111, "bitch"),             // bumped from B+Ih → Y+mod+Aw
    (0b1111, 0b11111, "luck"),              // bumped from L+Ah → Y+mod+Oy
];
