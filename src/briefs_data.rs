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
    (0b0000, 0b00001, "you"),               // ergo #1 val=28787591 (natural: Y+UW)
    (0b0000, 0b00010, "and"),               // ergo #2 val=21145876 (natural: N+AH)
    (0b0000, 0b00100, "that"),              // ergo #3 val=20407484 (natural: DH+AE)
    (0b0001, 0b00001, "to"),                // ergo #4 val=17099834 (natural: T+UW)
    (0b0010, 0b00001, "what"),              // ergo #5 val=13800328 (natural: W+AH)
    (0b0001, 0b00010, "it"),                // ergo #6 val=13631703 (natural: T+IH)
    (0b0010, 0b00010, "this"),              // ergo #7 val=11479576 (natural: DH+IH)
    (0b0000, 0b10000, "the"),               // pinned
    (0b0000, 0b01000, "for"),               // ergo #8 val=10348120 (natural: F+AO)
    (0b0100, 0b00001, "just"),              // ergo #9 val=10003176 (natural: JH+AH)
    (0b0100, 0b00010, "have"),              // ergo #10 val=9528020 (natural: HH+AE)
    (0b0000, 0b00011, "your"),              // ergo #11 val=9221890 (natural: Y+AO)
    (0b0001, 0b00100, "of"),                // ergo #12 val=8915110 (natural: V+AH)
    (0b0010, 0b00100, "not"),               // ergo #13 val=8524546 (natural: N+AA)
    (0b1000, 0b00001, "can"),               // ergo #14 val=7652236 (natural: K+AE)
    (0b1000, 0b00010, "with"),              // ergo #15 val=7613954 (natural: W+IH)
    (0b0100, 0b00100, "about"),             // ergo #16 val=7462044 (natural: B+AH)
    (0b0000, 0b00110, "is"),                // ergo #17 val=7400675 (natural: Z+IH)
    (0b0001, 0b01000, "in"),                // ergo #18 val=7337058 (natural: N+IH)
    (0b0010, 0b01000, "but"),               // ergo #19 val=7262924 (natural: B+AH)
    (0b0001, 0b10000, "we"),                // ergo #20 val=6755687 (natural: W+IY)
    (0b0010, 0b10000, "me"),                // ergo #21 val=6444985 (natural: M+IY)
    (0b0000, 0b10001, "there"),             // ergo #22 val=6297056 (natural: DH+EH)
    (0b0000, 0b10010, "here"),              // ergo #23 val=6277182 (natural: HH+IY)
    (0b0011, 0b00001, "like"),              // ergo #24 val=5966054 (natural: L+AY)
    (0b0011, 0b00010, "want"),              // ergo #25 val=5852535 (natural: W+AA)
    (0b0001, 0b00011, "get"),               // ergo #26 val=5766386 (natural: G+EH)
    (0b0010, 0b00011, "think"),             // ergo #27 val=5518419 (natural: TH+IH)
    (0b1000, 0b00100, "he"),                // ergo #28 val=5516364 (natural: HH+IY)
    (0b0000, 0b00101, "something"),         // ergo #29 val=5193190 (natural: S+AH)
    (0b0100, 0b01000, "right"),             // ergo #30 val=5153642 (natural: R+AY)
    (0b0100, 0b10000, "from"),              // ergo #31 val=5128746 (natural: F+AH)
    (0b0000, 0b10100, "my"),                // ergo #32 val=4938948 (natural: M+AY)
    (0b0110, 0b00001, "him"),               // ergo #33 val=4862118 (natural: HH+IH)
    (0b0110, 0b00010, "on"),                // ergo #34 val=4821861 (natural: N+AA)
    (0b0100, 0b00011, "one"),               // ergo #35 val=4527912 (natural: W+AH)
    (0b0011, 0b00100, "do"),                // ergo #36 val=4419883 (natural: D+UW)
    (0b0001, 0b00110, "come"),              // ergo #37 val=4407838 (natural: K+AH)
    (0b0010, 0b00110, "no"),                // ergo #38 val=4374975 (natural: N+OW)
    (0b0001, 0b10001, "well"),              // ergo #39 val=4319818 (natural: W+EH)
    (0b0010, 0b10001, "be"),                // ergo #40 val=4210868 (natural: B+IY)
    (0b0001, 0b10010, "are"),               // ergo #41 val=4203821 (natural: R+AA)
    (0b0010, 0b10010, "will"),              // ergo #42 val=3939614 (natural: W+IH)
    (0b1000, 0b01000, "know"),              // ergo #43 val=3892394 (natural: N+OW)
    (0b0000, 0b01010, "gonna"),             // ergo #44 val=3564570 (natural: G+AA)
    (0b0000, 0b01100, "all"),               // ergo #45 val=3544700 (natural: L+AO)
    (0b1000, 0b10000, "people"),            // ergo #46 val=3543660 (natural: P+IY)
    (0b0000, 0b11000, "because"),           // ergo #47 val=3520448 (natural: B+IH)
    (0b0101, 0b00001, "good"),              // ergo #48 val=3483460 (natural: G+UH)
    (0b0101, 0b00010, "little"),            // ergo #49 val=3478088 (natural: L+IH)
    (0b1000, 0b00011, "so"),                // ergo #50 val=3434152 (natural: S+OW)
    (0b0100, 0b00110, "let"),               // L+EH
    (0b0011, 0b00011, "anything"),          // bumped from N+Eh → N+Iy
    (0b0011, 0b01000, "please"),            // P+IY
    (0b0010, 0b00101, "remember"),          // R+IH
    (0b0001, 0b10100, "again"),             // G+AH
    (0b0000, 0b10011, "maybe"),             // bumped from M+Ey → M+-
    (0b0110, 0b00100, "actually"),          // bumped from K+Ae → K+Ey
    (0b0010, 0b10100, "give"),              // G+IH
    (0b0100, 0b10001, "next"),              // bumped from N+Eh → D+Eh
    (0b0100, 0b10010, "anyone"),            // bumped from N+Eh → Z+Eh
    (0b0001, 0b00101, "today"),             // bumped from T+Ah → R+Ah
    (0b0011, 0b10000, "under"),             // bumped from N+Ah → -+mod+Iy
    (0b0110, 0b00011, "understand"),        // bumped from N+Ah → N+Ey
    (0b0001, 0b01010, "where"),             // bumped from W+Eh → W+Ah
    (0b1010, 0b00001, "take"),              // bumped from T+Ey → T+Ow
    (0b0110, 0b01000, "up"),                // bumped from P+Ah → P+Ey
    (0b1010, 0b00010, "sorry"),             // bumped from S+Aa → S+Ow
    (0b1100, 0b00001, "at"),                // bumped from T+Ae → T+Ao
    (0b1100, 0b00010, "some"),              // bumped from S+Ah → S+Ao
    (0b0010, 0b11000, "before"),            // B+IH
    (0b1000, 0b10010, "as"),                // Z+AE
    (0b0001, 0b10011, "must"),              // M+AH
    (0b1000, 0b00110, "love"),              // bumped from L+Ah → L+Ae
    (0b0001, 0b11000, "believe"),           // bumped from B+Ih → B+Ah
    (0b0011, 0b00110, "last"),              // bumped from L+Ae → L+Iy
    (0b0010, 0b10011, "morning"),           // bumped from M+Ao → M+Ih
    (0b0000, 0b01001, "family"),            // bumped from F+Ae → F+-
    (0b0100, 0b10100, "exactly"),           // bumped from G+Ih → G+Eh
    (0b0101, 0b00100, "excuse"),            // bumped from K+Ih → K+Aa
    (0b0010, 0b01010, "woman"),             // bumped from W+Uh → W+Ih
    (0b0010, 0b01100, "enough"),            // bumped from N+Ih → Th+Ih
    (0b1000, 0b10001, "different"),         // bumped from D+Ih → D+Ae
    (0b0000, 0b00111, "haven"),             // bumped from H+Ey → H+-
    (0b0001, 0b01100, "somebody"),          // bumped from S+Ah → Th+Ah
    (0b0100, 0b00101, "second"),            // bumped from S+Eh → R+Eh
    (0b0110, 0b10000, "name"),              // bumped from N+Ey → -+mod+Ey
    (0b0011, 0b10001, "dad"),               // bumped from D+Ae → D+Iy
    (0b0000, 0b10110, "trust"),             // bumped from T+Ah → L+mod+-
    (0b0011, 0b10010, "seen"),              // bumped from S+Iy → Z+Iy
    (0b0010, 0b00111, "his"),               // HH+IH
    (0b0100, 0b01010, "when"),              // W+EH
    (0b1001, 0b00001, "time"),              // T+AY
    (0b1010, 0b00100, "okay"),              // K+OW
    (0b0101, 0b00011, "never"),             // bumped from N+Eh → N+Aa
    (0b0010, 0b01001, "if"),                // F+IH
    (0b0000, 0b10101, "then"),              // bumped from Dh+Eh → Dh+-
    (0b0111, 0b00001, "tell"),              // bumped from T+Eh → T+Er
    (0b1001, 0b00010, "someone"),           // bumped from S+Ah → S+Ay
    (0b0111, 0b00010, "still"),             // bumped from S+Ih → S+Er
    (0b0100, 0b10011, "much"),              // bumped from M+Ah → M+Eh
    (0b0110, 0b00110, "listen"),            // bumped from L+Ih → L+Ey
    (0b0101, 0b01000, "probably"),          // P+AA
    (0b0001, 0b00111, "hello"),             // HH+AH
    (0b1100, 0b00100, "call"),              // K+AO
    (0b0100, 0b11000, "brother"),           // bumped from B+Ah → B+Eh
    (0b0101, 0b10000, "start"),             // bumped from S+Aa → -+mod+Aa
    (0b0001, 0b01001, "tonight"),           // bumped from T+Ah → F+Ah
    (0b0001, 0b10110, "until"),             // bumped from N+Ah → L+mod+Ah
    (0b0100, 0b01100, "question"),          // bumped from K+Eh → Th+Eh
    (0b1000, 0b10100, "against"),           // bumped from G+Ah → G+Ae
    (0b1000, 0b00101, "ready"),             // bumped from R+Eh → R+Ae
    (0b0010, 0b10110, "since"),             // bumped from S+Ih → L+mod+Ih
    (0b0110, 0b10001, "stay"),              // bumped from S+Ey → D+Ey
    (0b0110, 0b10010, "crazy"),             // bumped from K+Ey → Z+Ey
    (0b0011, 0b00101, "wrong"),             // bumped from R+Ao → R+Iy
    (0b0011, 0b10100, "gotta"),             // bumped from G+Aa → G+Iy
    (0b1000, 0b11000, "back"),              // B+AE
    (0b1010, 0b00011, "only"),              // N+OW
    (0b1100, 0b00011, "nothing"),           // bumped from N+Ah → N+Ao
    (0b1000, 0b10011, "man"),               // M+AE
    (0b1000, 0b01100, "thank"),             // TH+AE
    (0b0100, 0b00111, "help"),              // HH+EH
    (0b0011, 0b10011, "mean"),              // M+IY
    (0b0110, 0b10100, "great"),             // G+EY
    (0b0101, 0b00110, "already"),           // bumped from L+Ao → L+Aa
    (0b0000, 0b01011, "sure"),              // bumped from Sh+Uh → Sh+-
    (0b0111, 0b00100, "around"),            // bumped from N+Er → K+Er
    (0b1010, 0b01000, "problem"),           // bumped from P+Aa → P+Ow
    (0b1100, 0b01000, "place"),             // bumped from P+Ey → P+Ao
    (0b0100, 0b01001, "friend"),            // F+EH
    (0b0001, 0b10101, "their"),             // bumped from Dh+Eh → Dh+Ah
    (0b1001, 0b00100, "kind"),              // K+AY
    (0b1000, 0b01010, "without"),           // bumped from W+Ih → W+Ae
    (0b0010, 0b10101, "other"),             // bumped from Dh+Ah → Dh+Ih
    (0b0011, 0b01010, "away"),              // bumped from W+Ah → W+Iy
    (0b0011, 0b11000, "big"),               // bumped from B+Ih → B+Iy
    (0b1010, 0b10000, "moment"),            // bumped from M+Ow → -+mod+Ow
    (0b0101, 0b10001, "doctor"),            // D+AA
    (0b1100, 0b10000, "almost"),            // bumped from L+Ao → -+mod+Ao
    (0b0100, 0b10110, "anymore"),           // bumped from N+Eh → L+mod+Eh
    (0b0011, 0b01100, "three"),             // TH+IY
    (0b0110, 0b00101, "same"),              // bumped from S+Ey → R+Ey
    (0b0101, 0b10010, "hospital"),          // bumped from H+Aa → Z+Aa
    (0b0000, 0b01110, "couple"),            // bumped from K+Ah → Ng+-
    (0b0000, 0b11010, "head"),              // bumped from H+Eh → W+mod+-
    (0b0000, 0b11100, "such"),              // bumped from S+Ah → Th+mod+-
    (0b0000, 0b11001, "very"),              // bumped from V+Eh → V+-
    (0b0100, 0b10101, "them"),              // DH+EH
    (0b1001, 0b00011, "need"),              // bumped from N+Iy → N+Ay
    (0b1100, 0b00110, "always"),            // L+AO
    (0b0111, 0b00011, "into"),              // bumped from N+Ih → N+Er
    (0b0110, 0b10011, "make"),              // M+EY
    (0b1011, 0b00010, "see"),               // bumped from S+Iy → S+Uw
    (0b1110, 0b00010, "stop"),              // bumped from S+Aa → S+Uh
    (0b1000, 0b01001, "after"),             // F+AE
    (0b1011, 0b00001, "together"),          // bumped from T+Ah → T+Uw
    (0b1110, 0b00001, "tomorrow"),          // bumped from T+Ah → T+Uh
    (0b0101, 0b10100, "god"),               // G+AA
    (0b0110, 0b01010, "wait"),              // W+EY
    (0b1001, 0b01000, "night"),             // bumped from N+Ay → P+Ay
    (0b0111, 0b01000, "pretty"),            // bumped from P+Ih → P+Er
    (0b1010, 0b00110, "long"),              // bumped from L+Ao → L+Ow
    (0b1000, 0b00111, "husband"),           // bumped from H+Ah → H+Ae
    (0b0011, 0b01001, "feel"),              // F+IY
    (0b0110, 0b11000, "baby"),              // B+EY
    (0b1001, 0b10000, "nice"),              // bumped from N+Ay → -+mod+Ay
    (0b0011, 0b00111, "hold"),              // bumped from H+Ow → H+Iy
    (0b1000, 0b10110, "am"),                // bumped from M+Ae → L+mod+Ae
    (0b1010, 0b10001, "nobody"),            // bumped from N+Ow → D+Ow
    (0b1010, 0b10010, "ok"),                // bumped from K+Ow → Z+Ow
    (0b0001, 0b01011, "sometimes"),         // bumped from S+Ah → Sh+Ah
    (0b0010, 0b01011, "minute"),            // bumped from M+Ih → Sh+Ih
    (0b1100, 0b10001, "drink"),             // bumped from D+Ih → D+Ao
    (0b0001, 0b01110, "alone"),             // bumped from L+Ah → Ng+Ah
    (0b0001, 0b11010, "trouble"),           // bumped from T+Ah → W+mod+Ah
    (0b0010, 0b01110, "inside"),            // bumped from N+Ih → Ng+Ih
    (0b0010, 0b11010, "kill"),              // bumped from K+Ih → W+mod+Ih
    (0b0101, 0b00101, "party"),             // bumped from P+Aa → R+Aa
    (0b0010, 0b11100, "himself"),           // bumped from H+Ih → Th+mod+Ih
    (0b1100, 0b10010, "story"),             // bumped from S+Ao → Z+Ao
    (0b0001, 0b11100, "number"),            // bumped from N+Ah → Th+mod+Ah
    (0b0111, 0b10000, "sir"),               // bumped from S+Er → -+mod+Er
    (0b0000, 0b01101, "become"),            // bumped from B+Ih → Ch+-
    (0b0110, 0b01100, "explain"),           // bumped from K+Ih → Th+Ey
    (0b0011, 0b10110, "sleep"),             // bumped from S+Iy → L+mod+Iy
    (0b0000, 0b10111, "interested"),        // bumped from N+Ih → H+mod+-
    (0b1010, 0b10100, "go"),                // G+OW
    (0b1101, 0b00001, "out"),               // T+AW
    (0b1110, 0b00100, "could"),             // K+UH
    (0b0001, 0b11001, "everyone"),          // bumped from V+Eh → V+Ah
    (0b0101, 0b10011, "money"),             // bumped from M+Ah → M+Aa
    (0b0011, 0b10101, "these"),             // DH+IY
    (0b1101, 0b00010, "us"),                // bumped from S+Ah → S+Aw
    (0b0010, 0b11001, "everybody"),         // bumped from V+Eh → V+Ih
    (0b1100, 0b00101, "or"),                // R+AO
    (0b1001, 0b00110, "life"),              // L+AY
    (0b0110, 0b00111, "hey"),               // HH+EY
    (0b1011, 0b00100, "course"),            // bumped from K+Ao → K+Uw
    (0b1000, 0b10101, "than"),              // DH+AE
    (0b0101, 0b01010, "world"),             // bumped from W+Er → W+Aa
    (0b0111, 0b00110, "leave"),             // bumped from L+Iy → L+Er
    (0b0110, 0b01001, "fine"),              // bumped from F+Ay → F+Ey
    (0b1001, 0b10001, "idea"),              // D+AY
    (0b0101, 0b11000, "between"),           // bumped from B+Ih → B+Aa
    (0b1001, 0b10010, "might"),             // bumped from M+Ay → Z+Ay
    (0b0100, 0b01011, "welcome"),           // bumped from W+Eh → Sh+Eh
    (0b0100, 0b01110, "else"),              // bumped from L+Eh → Ng+Eh
    (0b0100, 0b11010, "anybody"),           // bumped from N+Eh → W+mod+Eh
    (0b0100, 0b11100, "anyway"),            // bumped from N+Eh → Th+mod+Eh
    (0b0101, 0b01100, "car"),               // bumped from K+Aa → Th+Aa
    (0b0001, 0b01101, "son"),               // bumped from S+Ah → Ch+Ah
    (0b0001, 0b10111, "company"),           // bumped from K+Ah → H+mod+Ah
    (0b0111, 0b10001, "dead"),              // bumped from D+Eh → D+Er
    (0b0010, 0b01101, "mr"),                // bumped from M+Ih → Ch+Ih
    (0b0010, 0b10111, "sister"),            // bumped from S+Ih → H+mod+Ih
    (0b1010, 0b00101, "real"),              // bumped from R+Iy → R+Ow
    (0b1100, 0b10100, "water"),             // bumped from W+Ao → G+Ao
    (0b0111, 0b10010, "perfect"),           // bumped from P+Er → Z+Er
    (0b0110, 0b10110, "end"),               // bumped from N+Eh → L+mod+Ey
    (0b0100, 0b11001, "everything"),        // V+EH
    (0b0110, 0b10101, "they"),              // DH+EY
    (0b0011, 0b01011, "she"),               // SH+IY
    (0b1100, 0b10011, "more"),              // M+AO
    (0b1011, 0b00011, "any"),               // bumped from N+Eh → N+Uw
    (0b1110, 0b00011, "another"),           // bumped from N+Ah → N+Uh
    (0b1010, 0b10011, "important"),         // bumped from M+Ih → M+Ow
    (0b0101, 0b01001, "father"),            // F+AA
    (0b1101, 0b00100, "our"),               // bumped from -+Aw → K+Aw
    (0b1011, 0b01000, "too"),               // bumped from T+Uw → P+Uw
    (0b1010, 0b01010, "way"),               // bumped from W+Ey → W+Ow
    (0b1100, 0b01010, "whatever"),          // bumped from W+Ah → W+Ao
    (0b1110, 0b01000, "police"),            // bumped from P+Ah → P+Uh
    (0b0111, 0b10100, "girl"),              // G+ER
    (0b0101, 0b00111, "happy"),             // bumped from H+Ae → H+Aa
    (0b1011, 0b10000, "two"),               // bumped from T+Uw → -+mod+Uw
    (0b1000, 0b01011, "matter"),            // bumped from M+Ae → Sh+Ae
    (0b1010, 0b11000, "behind"),            // bumped from B+Ih → B+Ow
    (0b1100, 0b11000, "bring"),             // bumped from B+Ih → B+Ao
    (0b1000, 0b01110, "ask"),               // bumped from S+Ae → Ng+Ae
    (0b0100, 0b01101, "care"),              // bumped from K+Eh → Ch+Eh
    (0b0100, 0b10111, "parents"),           // bumped from P+Eh → H+mod+Eh
    (0b1001, 0b00101, "try"),               // bumped from T+Ay → R+Ay
    (0b0101, 0b10110, "heart"),             // bumped from H+Aa → L+mod+Aa
    (0b1000, 0b11010, "captain"),           // bumped from K+Ae → W+mod+Ae
    (0b1000, 0b11100, "stand"),             // bumped from S+Ae → Th+mod+Ae
    (0b0111, 0b00101, "reason"),            // bumped from R+Iy → R+Er
    (0b1001, 0b10100, "live"),              // bumped from L+Ay → G+Ay
    (0b1100, 0b01100, "door"),              // bumped from D+Ao → Th+Ao
    (0b1010, 0b01100, "whole"),             // bumped from H+Ow → Th+Ow
    (0b0011, 0b01110, "meet"),              // bumped from M+Iy → Ng+Iy
    (0b0011, 0b11010, "least"),             // bumped from L+Iy → W+mod+Iy
    (0b1110, 0b10000, "million"),           // bumped from M+Ih → -+mod+Uh
    (0b0000, 0b11011, "situation"),         // bumped from S+Ih → Zh+-
    (0b0000, 0b11110, "somewhere"),         // bumped from S+Ah → Ng+mod+-
    (0b0011, 0b11100, "decided"),           // bumped from D+Ih → Th+mod+Iy
    (0b1110, 0b00110, "look"),              // L+UH
    (0b1101, 0b00011, "now"),               // N+AW
    (0b0011, 0b11001, "even"),              // V+IY
    (0b1001, 0b01010, "why"),               // W+AY
    (0b0111, 0b01010, "work"),              // W+ER
    (0b1001, 0b10011, "myself"),            // M+AY
    (0b1000, 0b11001, "every"),             // bumped from V+Eh → V+Ae
    (0b0111, 0b10011, "mother"),            // bumped from M+Ah → M+Er
    (0b1010, 0b00111, "home"),              // HH+OW
    (0b1001, 0b11000, "by"),                // B+AY
    (0b1011, 0b00110, "old"),               // bumped from L+Ow → L+Uw
    (0b1100, 0b01001, "off"),               // F+AO
    (0b1100, 0b00111, "house"),             // bumped from H+Aw → H+Ao
    (0b1101, 0b01000, "person"),            // bumped from P+Er → P+Aw
    (0b1010, 0b01001, "forget"),            // bumped from F+Er → F+Ow
    (0b0111, 0b11000, "bad"),               // bumped from B+Ae → B+Er
    (0b1011, 0b10001, "school"),            // bumped from S+Uw → D+Uw
    (0b1011, 0b10010, "stupid"),            // bumped from S+Uw → Z+Uw
    (0b0101, 0b10101, "possible"),          // bumped from P+Aa → Dh+Aa
    (0b0001, 0b11011, "country"),           // bumped from K+Ah → Zh+Ah
    (0b1000, 0b01101, "hand"),              // bumped from H+Ae → Ch+Ae
    (0b1110, 0b10001, "day"),               // bumped from D+Ey → D+Uh
    (0b0001, 0b11110, "control"),           // bumped from K+Ah → Ng+mod+Ah
    (0b0010, 0b11011, "information"),       // bumped from N+Ih → Zh+Ih
    (0b0010, 0b11110, "security"),          // bumped from S+Ih → Ng+mod+Ih
    (0b0110, 0b01011, "shit"),              // bumped from Sh+Ih → Sh+Ey
    (0b0111, 0b01100, "perhaps"),           // bumped from P+Er → Th+Er
    (0b0000, 0b01111, "yet"),               // bumped from Y+Eh → Y+-
    (0b1001, 0b01100, "while"),             // bumped from W+Ay → Th+Ay
    (0b1101, 0b10000, "outside"),           // bumped from T+Aw → -+mod+Aw
    (0b0011, 0b01101, "chance"),            // bumped from Ch+Ae → Ch+Iy
    (0b1010, 0b10110, "close"),             // bumped from K+Ow → L+mod+Ow
    (0b1100, 0b10110, "daughter"),          // bumped from D+Ao → L+mod+Ao
    (0b0110, 0b01110, "play"),              // bumped from P+Ey → Ng+Ey
    (0b0011, 0b10111, "speak"),             // bumped from S+Iy → H+mod+Iy
    (0b0110, 0b11010, "face"),              // bumped from F+Ey → W+mod+Ey
    (0b0000, 0b11101, "careful"),           // bumped from K+Eh → Jh+-
    (0b1110, 0b10010, "blood"),             // bumped from B+Ah → Z+Uh
    (0b0110, 0b11100, "able"),              // bumped from B+Ey → Th+mod+Ey
    (0b1000, 0b10111, "american"),          // bumped from M+Ah → H+mod+Ae
    (0b0111, 0b00111, "her"),               // HH+ER
    (0b0111, 0b01001, "first"),             // F+ER
    (0b1101, 0b10001, "down"),              // D+AW
    (0b1001, 0b01001, "find"),              // F+AY
    (0b1111, 0b00010, "say"),               // bumped from S+Ey → S+Oy
    (0b1111, 0b00001, "talk"),              // bumped from T+Ao → T+Oy
    (0b1010, 0b10101, "those"),             // DH+OW
    (0b0110, 0b11001, "ever"),              // bumped from V+Eh → V+Ey
    (0b1101, 0b00110, "lot"),               // bumped from L+Aa → L+Aw
    (0b1001, 0b00111, "happen"),            // bumped from H+Ae → H+Ay
    (0b0100, 0b11011, "president"),         // bumped from P+Eh → Zh+Eh
    (0b0101, 0b01011, "mom"),               // bumped from M+Aa → Sh+Aa
    (0b0100, 0b11110, "hell"),              // bumped from H+Eh → Ng+mod+Eh
    (0b0001, 0b01111, "wonderful"),         // bumped from W+Ah → Y+Ah
    (0b0001, 0b11101, "completely"),        // bumped from K+Ah → Jh+Ah
    (0b1011, 0b00101, "room"),              // R+UW
    (0b0101, 0b01110, "promise"),           // bumped from P+Aa → Ng+Aa
    (0b0101, 0b11010, "part"),              // bumped from P+Aa → W+mod+Aa
    (0b1011, 0b10100, "move"),              // bumped from M+Uw → G+Uw
    (0b0010, 0b01111, "miss"),              // bumped from M+Ih → Y+Ih
    (0b0110, 0b01101, "change"),            // CH+EY
    (0b0010, 0b11101, "serious"),           // bumped from S+Ih → Jh+Ih
    (0b0111, 0b10110, "heard"),             // bumped from H+Er → L+mod+Er
    (0b1001, 0b10110, "wife"),              // bumped from W+Ay → L+mod+Ay
    (0b0101, 0b11100, "body"),              // bumped from B+Aa → Th+mod+Aa
    (0b1110, 0b10100, "guess"),             // bumped from G+Eh → G+Uh
    (0b1110, 0b00101, "relationship"),      // bumped from R+Iy → R+Uh
    (0b1100, 0b10101, "alright"),           // bumped from L+Ao → Dh+Ao
    (0b0110, 0b10111, "lady"),              // bumped from L+Ey → H+mod+Ey
    (0b1101, 0b10010, "scared"),            // bumped from S+Eh → Z+Aw
    (0b0100, 0b01111, "yes"),               // Y+EH
    (0b1110, 0b01010, "would"),             // W+UH
    (0b1011, 0b11000, "beautiful"),         // B+UW
    (0b1011, 0b10011, "music"),             // M+UW
    (0b1111, 0b00100, "keep"),              // bumped from K+Iy → K+Oy
    (0b1110, 0b10011, "many"),              // bumped from M+Eh → M+Uh
    (0b1011, 0b01010, "which"),             // bumped from W+Ih → W+Uw
    (0b1011, 0b01100, "through"),           // TH+UW
    (0b0100, 0b11101, "gentlemen"),         // JH+EH
    (0b1110, 0b11000, "absolutely"),        // bumped from B+Ae → B+Uh
    (0b1001, 0b10101, "quite"),             // bumped from K+Ay → Dh+Ay
    (0b0101, 0b01101, "chuckles"),          // bumped from Ch+Ah → Ch+Aa
    (0b1101, 0b00101, "run"),               // bumped from R+Ah → R+Aw
    (0b1010, 0b01011, "phone"),             // bumped from F+Ow → Sh+Ow
    (0b1101, 0b10100, "ago"),               // bumped from G+Ah → G+Aw
    (0b1010, 0b01110, "hope"),              // bumped from H+Ow → Ng+Ow
    (0b0011, 0b11011, "secret"),            // bumped from S+Iy → Zh+Iy
    (0b0101, 0b11001, "evidence"),          // bumped from V+Eh → V+Aa
    (0b0111, 0b10101, "turn"),              // bumped from T+Er → Dh+Er
    (0b1000, 0b11011, "accident"),          // bumped from K+Ae → Zh+Ae
    (0b1100, 0b01011, "government"),        // bumped from G+Ah → Sh+Ao
    (0b1100, 0b01110, "uncle"),             // bumped from Ng+Ah → Ng+Ao
    (0b1110, 0b01100, "impossible"),        // bumped from M+Ih → Th+Uh
    (0b1010, 0b11010, "city"),              // bumped from S+Ih → W+mod+Ow
    (0b1100, 0b11010, "detective"),         // bumped from D+Ih → W+mod+Ao
    (0b1010, 0b11100, "christmas"),         // bumped from K+Ih → Th+mod+Ow
    (0b1100, 0b11100, "office"),            // bumped from F+Ao → Th+mod+Ao
    (0b1000, 0b11110, "difficult"),         // bumped from D+Ih → Ng+mod+Ae
    (0b0101, 0b10111, "apartment"),         // bumped from P+Ah → H+mod+Aa
    (0b0011, 0b11110, "expect"),            // bumped from K+Ih → Ng+mod+Iy
    (0b1000, 0b01111, "yeah"),              // Y+AE
    (0b1010, 0b11001, "over"),              // V+OW
    (0b1011, 0b00111, "who"),               // HH+UW
    (0b1111, 0b00011, "an"),                // bumped from N+Ae → N+Oy
    (0b1101, 0b10011, "mind"),              // bumped from M+Ay → M+Aw
    (0b1101, 0b01010, "once"),              // bumped from W+Ah → W+Aw
    (0b1110, 0b00111, "hard"),              // bumped from H+Aa → H+Uh
    (0b1111, 0b01000, "open"),              // bumped from P+Ow → P+Oy
    (0b1011, 0b01001, "afraid"),            // bumped from F+Ah → F+Uw
    (0b1101, 0b11000, "both"),              // bumped from B+Ow → B+Aw
    (0b1110, 0b01001, "front"),             // bumped from F+Ah → F+Uh
    (0b1111, 0b10000, "point"),             // bumped from P+Oy → -+mod+Oy
    (0b1011, 0b10110, "human"),             // bumped from H+Uw → L+mod+Uw
    (0b0011, 0b01111, "yesterday"),         // bumped from Y+Eh → Y+Iy
    (0b1001, 0b01011, "finally"),           // bumped from F+Ay → Sh+Ay
    (0b0110, 0b11011, "case"),              // bumped from K+Ey → Zh+Ey
    (0b0110, 0b11110, "break"),             // bumped from B+Ey → Ng+mod+Ey
    (0b0111, 0b01011, "worry"),             // bumped from W+Er → Sh+Er
    (0b1101, 0b01100, "terrible"),          // bumped from T+Eh → Th+Aw
    (0b1000, 0b11101, "cannot"),            // bumped from K+Ae → Jh+Ae
    (0b1010, 0b01101, "honey"),             // bumped from H+Ah → Ch+Ow
    (0b1100, 0b01101, "amazing"),           // bumped from M+Ah → Ch+Ao
    (0b1001, 0b01110, "protect"),           // bumped from P+Ah → Ng+Ay
    (0b1100, 0b11001, "sit"),               // bumped from S+Ih → V+Ao
    (0b1001, 0b11010, "send"),              // bumped from S+Eh → W+mod+Ay
    (0b0111, 0b01110, "girlfriend"),        // bumped from G+Er → Ng+Er
    (0b1001, 0b11100, "attention"),         // bumped from T+Ah → Th+mod+Ay
    (0b0011, 0b11101, "street"),            // bumped from S+Iy → Jh+Iy
    (0b0000, 0b11111, "along"),             // bumped from L+Ah → Y+mod+-
    (0b1110, 0b10110, "late"),              // bumped from L+Ey → L+mod+Uh
    (0b1010, 0b10111, "master"),            // bumped from M+Ae → H+mod+Ow
    (0b1100, 0b10111, "present"),           // bumped from P+Eh → H+mod+Ao
    (0b0111, 0b11010, "death"),             // bumped from D+Eh → W+mod+Er
    (0b0111, 0b11100, "clear"),             // bumped from K+Ih → Th+mod+Er
    (0b1101, 0b00111, "how"),               // HH+AW
    (0b1101, 0b01001, "found"),             // F+AW
    (0b1111, 0b00110, "also"),              // bumped from L+Ao → L+Oy
    (0b1111, 0b10001, "definitely"),        // bumped from D+Eh → D+Oy
    (0b0001, 0b11111, "stuff"),             // bumped from S+Ah → Y+mod+Ah
    (0b0010, 0b11111, "interesting"),       // bumped from N+Ih → Y+mod+Ih
    (0b1011, 0b10101, "new"),               // bumped from N+Uw → Dh+Uw
    (0b1001, 0b01101, "child"),             // CH+AY
    (0b0110, 0b01111, "year"),              // bumped from Y+Ih → Y+Ey
    (0b0101, 0b11011, "watch"),             // bumped from W+Aa → Zh+Aa
    (0b0101, 0b11110, "darling"),           // bumped from D+Aa → Ng+mod+Aa
    (0b0110, 0b11101, "dangerous"),         // bumped from D+Ey → Jh+Ey
    (0b1001, 0b11001, "kid"),               // bumped from K+Ih → V+Ay
    (0b0111, 0b01101, "fact"),              // bumped from F+Ae → Ch+Er
    (0b1111, 0b10010, "mistake"),           // bumped from M+Ih → Z+Oy
    (0b1110, 0b10101, "may"),               // bumped from M+Ey → Dh+Uh
    (0b1101, 0b10110, "building"),          // bumped from B+Ih → L+mod+Aw
    (0b1001, 0b10111, "six"),               // bumped from S+Ih → H+mod+Ay
    (0b0111, 0b11001, "funny"),             // bumped from F+Ah → V+Er
    (0b0111, 0b10111, "black"),             // bumped from B+Ae → H+mod+Er
    (0b1110, 0b01011, "should"),            // SH+UH
    (0b0101, 0b11101, "job"),               // JH+AA
    (0b0100, 0b11111, "special"),           // bumped from S+Eh → Y+mod+Eh
    (0b1111, 0b00101, "rest"),              // bumped from R+Eh → R+Oy
    (0b1011, 0b01011, "few"),               // bumped from F+Uw → Sh+Uw
    (0b1011, 0b01110, "truth"),             // bumped from T+Uw → Ng+Uw
    (0b1111, 0b10100, "guy"),               // bumped from G+Ay → G+Oy
    (0b1011, 0b11010, "true"),              // bumped from T+Uw → W+mod+Uw
    (0b1011, 0b11100, "soon"),              // bumped from S+Uw → Th+mod+Uw
    (0b0101, 0b01111, "young"),             // bumped from Y+Ah → Y+Aa
    (0b1100, 0b11011, "order"),             // bumped from R+Ao → Zh+Ao
    (0b1100, 0b11110, "lord"),              // bumped from L+Ao → Ng+mod+Ao
    (0b1110, 0b01110, "alive"),             // bumped from L+Ah → Ng+Uh
    (0b1101, 0b10101, "damn"),              // bumped from D+Ae → Dh+Aw
    (0b1110, 0b11010, "system"),            // bumped from S+Ih → W+mod+Uh
    (0b1010, 0b11011, "far"),               // bumped from F+Aa → Zh+Ow
    (0b1010, 0b11110, "totally"),           // bumped from T+Ow → Ng+mod+Ow
    (0b1110, 0b11100, "its"),               // bumped from T+Ih → Th+mod+Uh
    (0b1111, 0b10011, "most"),              // bumped from M+Ow → M+Oy
    (0b1111, 0b01010, "wanna"),             // bumped from W+Aa → W+Oy
    (0b1111, 0b11000, "bit"),               // bumped from B+Ih → B+Oy
    (0b1000, 0b11111, "answer"),            // bumped from N+Ae → Y+mod+Ae
    (0b0011, 0b11111, "report"),            // bumped from R+Iy → Y+mod+Iy
    (0b1010, 0b11101, "general"),           // bumped from Jh+Eh → Jh+Ow
    (0b1011, 0b01101, "lieutenant"),        // bumped from L+Uw → Ch+Uw
    (0b1001, 0b11011, "five"),              // bumped from F+Ay → Zh+Ay
    (0b1100, 0b11101, "jesus"),             // bumped from Jh+Iy → Jh+Ao
    (0b0111, 0b11011, "personal"),          // bumped from P+Er → Zh+Er
    (0b1001, 0b11110, "quiet"),             // bumped from K+Ay → Ng+mod+Ay
    (0b1101, 0b01011, "ahead"),             // bumped from H+Ah → Sh+Aw
    (0b1111, 0b01100, "imagine"),           // bumped from M+Ih → Th+Oy
    (0b1110, 0b01101, "america"),           // bumped from M+Ah → Ch+Uh
    (0b1101, 0b01110, "instead"),           // bumped from N+Ih → Ng+Aw
    (0b1010, 0b01111, "return"),            // bumped from R+Ih → Y+Ow
    (0b1100, 0b01111, "afternoon"),         // bumped from F+Ae → Y+Ao
    (0b1011, 0b11001, "hit"),               // bumped from H+Ih → V+Uw
    (0b1110, 0b11001, "screaming"),         // bumped from S+Iy → V+Uh
    (0b1101, 0b11010, "easy"),              // bumped from Z+Iy → W+mod+Aw
    (0b1101, 0b11100, "plan"),              // bumped from P+Ae → Th+mod+Aw
    (0b1011, 0b10111, "dinner"),            // bumped from D+Ih → H+mod+Uw
    (0b1110, 0b10111, "deal"),              // bumped from D+Iy → H+mod+Uh
    (0b0111, 0b11110, "straight"),          // bumped from S+Ey → Ng+mod+Er
    (0b0111, 0b01111, "yourself"),          // Y+ER
    (0b1111, 0b00111, "hear"),              // bumped from H+Iy → H+Oy
    (0b1111, 0b01001, "fuck"),              // bumped from F+Ah → F+Oy
    (0b0110, 0b11111, "strange"),           // bumped from S+Ey → Y+mod+Ey
    (0b1101, 0b01101, "check"),             // bumped from Ch+Eh → Ch+Aw
    (0b1001, 0b01111, "show"),              // bumped from Sh+Ow → Y+Ay
    (0b1101, 0b11001, "respect"),           // bumped from R+Ih → V+Aw
    (0b1001, 0b11101, "history"),           // bumped from H+Ih → Jh+Ay
    (0b1111, 0b10110, "except"),            // bumped from K+Ih → L+mod+Oy
    (0b1101, 0b10111, "sent"),              // bumped from S+Eh → H+mod+Aw
    (0b0111, 0b11101, "small"),             // bumped from S+Ao → Jh+Er
    (0b0101, 0b11111, "obviously"),         // bumped from B+Aa → Y+mod+Aa
    (0b1011, 0b11011, "future"),            // bumped from F+Uw → Zh+Uw
    (0b1111, 0b10101, "handle"),            // bumped from H+Ae → Dh+Oy
    (0b1110, 0b11011, "dear"),              // bumped from D+Ih → Zh+Uh
    (0b1011, 0b11110, "four"),              // bumped from F+Ao → Ng+mod+Uw
    (0b1110, 0b11110, "picture"),           // bumped from P+Ih → Ng+mod+Uh
    (0b1011, 0b01111, "use"),               // Y+UW
    (0b1111, 0b01011, "shut"),              // bumped from Sh+Ah → Sh+Oy
    (0b1010, 0b11111, "own"),               // bumped from N+Ow → Y+mod+Ow
    (0b1100, 0b11111, "sort"),              // bumped from S+Ao → Y+mod+Ao
    (0b1111, 0b01110, "wonder"),            // bumped from W+Ah → Ng+Oy
    (0b1110, 0b01111, "fun"),               // bumped from F+Ah → Y+Uh
    (0b1111, 0b11010, "anywhere"),          // bumped from N+Eh → W+mod+Oy
    (0b1101, 0b11011, "set"),               // bumped from S+Eh → Zh+Aw
    (0b1111, 0b11100, "shall"),             // bumped from Sh+Ae → Th+mod+Oy
    (0b1011, 0b11101, "position"),          // bumped from P+Ah → Jh+Uw
    (0b1110, 0b11101, "simple"),            // bumped from S+Ih → Jh+Uh
    (0b1101, 0b11110, "especially"),        // bumped from S+Ah → Ng+mod+Aw
    (0b1001, 0b11111, "mine"),              // bumped from M+Ay → Y+mod+Ay
    (0b0111, 0b11111, "word"),              // bumped from W+Er → Y+mod+Er
    (0b1111, 0b01101, "figure"),            // bumped from F+Ih → Ch+Oy
    (0b1101, 0b01111, "single"),            // bumped from S+Ih → Y+Aw
    (0b1111, 0b11001, "hurt"),              // bumped from H+Er → V+Oy
    (0b1101, 0b11101, "involved"),          // bumped from N+Ih → Jh+Aw
    (0b1111, 0b10111, "strong"),            // bumped from S+Ao → H+mod+Oy
    (0b1111, 0b11011, "wish"),              // bumped from W+Ih → Zh+Oy
    (0b1111, 0b11110, "hi"),                // bumped from H+Ay → Ng+mod+Oy
    (0b1011, 0b11111, "fight"),             // bumped from F+Ay → Y+mod+Uw
    (0b1110, 0b11111, "past"),              // bumped from P+Ae → Y+mod+Uh
    (0b1111, 0b01111, "week"),              // bumped from W+Iy → Y+Oy
    (0b1111, 0b11101, "boyfriend"),         // bumped from B+Oy → Jh+Oy
    (0b1101, 0b11111, "normal"),            // bumped from N+Ao → Y+mod+Aw
    (0b1111, 0b11111, "sound"),             // bumped from S+Aw → Y+mod+Oy
];
