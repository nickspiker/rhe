/// Auto-generated brief assignments. Edit and recompile to customize.
/// Format: (right_5bits, left_4bits, "word")
/// Right bits: I=0001 M=0010 R=0100 P=1000 T(mod)=10000
/// Left bits:  I=0001 M=0010 R=0100 P=1000
pub const BRIEFS: &[(u8, u8, &str)] = &[
    (0b00000, 0b0001, "you"),               // ergo #1 (natural: Y+UW)
    (0b00000, 0b0010, "i"),                 // ergo #2 (natural: -+AY)
    (0b00001, 0b0000, "to"),                // ergo #3 (natural: T+UW)
    (0b00010, 0b0000, "a"),                 // ergo #4 (natural: -+AH)
    (0b00000, 0b0100, "'s"),                // ergo #5 (natural: S+EH)
    (0b00100, 0b0000, "it"),                // ergo #6 (natural: T+IH)
    (0b00001, 0b0001, "and"),               // ergo #7 (natural: N+AH)
    (0b00001, 0b0010, "that"),              // ergo #8 (natural: DH+AE)
    (0b00010, 0b0001, "of"),                // ergo #9 (natural: V+AH)
    (0b00010, 0b0010, "is"),                // ergo #10 (natural: Z+IH)
    (0b10000, 0b0000, "the"),               // pinned
    (0b00000, 0b1000, "in"),                // ergo #11 (natural: N+IH)
    (0b01000, 0b0000, "what"),              // ergo #12 (natural: W+AH)
    (0b00000, 0b0011, "we"),                // ergo #13 (natural: W+IY)
    (0b00001, 0b0100, "me"),                // ergo #14 (natural: M+IY)
    (0b00010, 0b0100, "this"),              // ergo #15 (natural: DH+IH)
    (0b00011, 0b0000, "he"),                // ergo #16 (natural: HH+IY)
    (0b00100, 0b0001, "for"),               // ergo #17 (natural: F+AO)
    (0b00100, 0b0010, "my"),                // ergo #18 (natural: M+AY)
    (0b00000, 0b0110, "on"),                // ergo #19 (natural: N+AA)
    (0b00001, 0b1000, "have"),              // ergo #20 (natural: HH+AE)
    (0b00010, 0b1000, "your"),              // ergo #21 (natural: Y+AO)
    (0b00100, 0b0100, "do"),                // ergo #22 (natural: D+UW)
    (0b00110, 0b0000, "was"),               // ergo #23 (natural: W+AA)
    (0b01000, 0b0001, "'m"),                // ergo #24 (natural: M+AH)
    (0b01000, 0b0010, "no"),                // ergo #25 (natural: N+OW)
    (0b10000, 0b0001, "not"),               // ergo #26 (natural: N+AA)
    (0b10000, 0b0010, "be"),                // ergo #27 (natural: B+IY)
    (0b10001, 0b0000, "are"),               // ergo #28 (natural: R+AA)
    (0b10010, 0b0000, "don"),               // ergo #29 (natural: D+AA)
    (0b00001, 0b0011, "know"),              // ergo #30 (natural: N+OW)
    (0b00010, 0b0011, "can"),               // ergo #31 (natural: K+AE)
    (0b00011, 0b0001, "with"),              // ergo #32 (natural: W+IH)
    (0b00011, 0b0010, "but"),               // ergo #33 (natural: B+AH)
    (0b00000, 0b0101, "all"),               // ergo #34 (natural: L+AO)
    (0b00100, 0b1000, "so"),                // ergo #35 (natural: S+OW)
    (0b00101, 0b0000, "just"),              // ergo #36 (natural: JH+AH)
    (0b01000, 0b0100, "there"),             // ergo #37 (natural: DH+EH)
    (0b10000, 0b0100, "here"),              // ergo #38 (natural: HH+IY)
    (0b10100, 0b0000, "they"),              // ergo #39 (natural: DH+EY)
    (0b00001, 0b0110, "like"),              // ergo #40 (natural: L+AY)
    (0b00010, 0b0110, "get"),               // ergo #41 (natural: G+EH)
    (0b00011, 0b0100, "she"),               // ergo #42 (natural: SH+IY)
    (0b00100, 0b0011, "go"),                // ergo #43 (natural: G+OW)
    (0b00110, 0b0001, "if"),                // ergo #44 (natural: F+IH)
    (0b00110, 0b0010, "right"),             // ergo #45 (natural: R+AY)
    (0b10001, 0b0001, "out"),               // ergo #46 (natural: T+AW)
    (0b10001, 0b0010, "about"),             // ergo #47 (natural: B+AH)
    (0b10010, 0b0001, "up"),                // ergo #48 (natural: P+AH)
    (0b10010, 0b0010, "at"),                // ergo #49 (natural: T+AE)
    (0b00000, 0b1010, "him"),               // ergo #50 (natural: HH+IH)
    (0b00000, 0b1100, "oh"),                // bumped from -+Ow → -+Ao
    (0b00100, 0b0110, "come"),              // bumped from K+Ah → K+Ey
    (0b00010, 0b0101, "see"),               // bumped from S+Iy → S+Aa
    (0b10001, 0b0100, "did"),               // bumped from D+Ih → D+Eh
    (0b00110, 0b0100, "let"),               // L+EH
    (0b01010, 0b0000, "when"),              // bumped from W+Eh → W+-
    (0b00011, 0b1000, "an"),                // N+AE
    (0b00001, 0b0101, "take"),              // bumped from T+Ey → T+Aa
    (0b10100, 0b0001, "gonna"),             // bumped from G+Aa → G+Ah
    (0b00101, 0b0010, "really"),            // R+IH
    (0b00011, 0b0011, "need"),              // N+IY
    (0b01000, 0b0011, "people"),            // P+IY
    (0b11000, 0b0000, "because"),           // bumped from B+Ih → B+-
    (0b01000, 0b1000, "please"),            // bumped from P+Iy → P+Ae
    (0b10100, 0b0010, "give"),              // G+IH
    (0b10011, 0b0000, "am"),                // bumped from M+Ae → M+-
    (0b01100, 0b0000, "thing"),             // bumped from Th+Ih → Th+-
    (0b00101, 0b0001, "someone"),           // bumped from S+Ah → R+Ah
    (0b10000, 0b1000, "ask"),               // bumped from S+Ae → -+mod+Ae
    (0b10010, 0b0100, "else"),              // bumped from L+Eh → Z+Eh
    (0b10000, 0b0011, "seen"),              // bumped from S+Iy → -+mod+Iy
    (0b01010, 0b0001, "one"),               // W+AH
    (0b01010, 0b0010, "will"),              // W+IH
    (0b01100, 0b0010, "think"),             // TH+IH
    (0b10010, 0b1000, "as"),                // Z+AE
    (0b00010, 0b1010, "us"),                // bumped from S+Ah → S+Ow
    (0b11000, 0b0010, "been"),              // B+IH
    (0b00001, 0b1010, "tell"),              // bumped from T+Eh → T+Ow
    (0b00010, 0b1100, "some"),              // bumped from S+Ah → S+Ao
    (0b00111, 0b0000, "has"),               // bumped from H+Ae → H+-
    (0b00011, 0b0110, "never"),             // bumped from N+Eh → N+Ey
    (0b00110, 0b1000, "little"),            // bumped from L+Ih → L+Ae
    (0b00110, 0b0011, "love"),              // bumped from L+Ah → L+Iy
    (0b00001, 0b1100, "two"),               // bumped from T+Uw → T+Ao
    (0b10011, 0b0001, "much"),              // M+AH
    (0b10100, 0b0100, "god"),               // bumped from G+Aa → G+Eh
    (0b00000, 0b1001, "uh"),                // bumped from -+Ah → -+Ay
    (0b10011, 0b0010, "maybe"),             // bumped from M+Ey → M+Ih
    (0b11000, 0b0001, "before"),            // bumped from B+Ih → B+Ah
    (0b00100, 0b0101, "stop"),              // bumped from S+Aa → K+Aa
    (0b01100, 0b0001, "things"),            // bumped from Th+Ih → Th+Ah
    (0b10001, 0b1000, "does"),              // bumped from D+Ah → D+Ae
    (0b01000, 0b0110, "name"),              // bumped from N+Ey → P+Ey
    (0b10001, 0b0011, "done"),              // bumped from D+Ah → D+Iy
    (0b01001, 0b0000, "fine"),              // bumped from F+Ay → F+-
    (0b10000, 0b0110, "stay"),              // bumped from S+Ey → -+mod+Ey
    (0b00101, 0b0100, "wrong"),             // bumped from R+Ao → R+Eh
    (0b00000, 0b0111, "world"),             // bumped from W+Er → -+Er
    (0b10010, 0b0011, "meet"),              // bumped from M+Iy → Z+Iy
    (0b10110, 0b0000, "until"),             // bumped from N+Ah → L+mod+-
    (0b01010, 0b0100, "well"),              // W+EH
    (0b01001, 0b0001, "from"),              // F+AH
    (0b00111, 0b0010, "his"),               // HH+IH
    (0b00001, 0b1001, "time"),              // T+AY
    (0b00100, 0b1010, "okay"),              // K+OW
    (0b10101, 0b0000, "then"),              // bumped from Dh+Eh → Dh+-
    (0b00010, 0b1001, "say"),               // bumped from S+Ey → S+Ay
    (0b00010, 0b0111, "something"),         // bumped from S+Ah → S+Er
    (0b00011, 0b0101, "any"),               // bumped from N+Eh → N+Aa
    (0b10001, 0b0110, "day"),               // D+EY
    (0b10100, 0b1000, "again"),             // bumped from G+Ah → G+Ae
    (0b10011, 0b0100, "must"),              // bumped from M+Ah → M+Eh
    (0b00100, 0b1100, "call"),              // K+AO
    (0b00001, 0b0111, "talk"),              // bumped from T+Ao → T+Er
    (0b00110, 0b0110, "last"),              // bumped from L+Ae → L+Ey
    (0b11000, 0b0100, "better"),            // B+EH
    (0b01000, 0b0101, "place"),             // bumped from P+Ey → P+Aa
    (0b10100, 0b0011, "guy"),               // bumped from G+Ay → G+Iy
    (0b00111, 0b0001, "hello"),             // HH+AH
    (0b01100, 0b0100, "thanks"),            // bumped from Th+Ae → Th+Eh
    (0b01001, 0b0010, "enough"),            // bumped from N+Ih → F+Ih
    (0b10010, 0b0110, "came"),              // bumped from K+Ey → Z+Ey
    (0b10110, 0b0001, "another"),           // bumped from N+Ah → L+mod+Ah
    (0b00101, 0b1000, "remember"),          // bumped from R+Ih → R+Ae
    (0b10110, 0b0010, "kill"),              // bumped from K+Ih → L+mod+Ih
    (0b10000, 0b0101, "car"),               // bumped from K+Aa → -+mod+Aa
    (0b00101, 0b0011, "real"),              // R+IY
    (0b11000, 0b1000, "back"),              // B+AE
    (0b01010, 0b1000, "where"),             // bumped from W+Eh → W+Ae
    (0b10011, 0b1000, "man"),               // M+AE
    (0b00011, 0b1010, "only"),              // N+OW
    (0b10011, 0b0011, "mean"),              // M+IY
    (0b01100, 0b1000, "thank"),             // TH+AE
    (0b01011, 0b0000, "sure"),              // bumped from Sh+Uh → Sh+-
    (0b00111, 0b0100, "help"),              // HH+EH
    (0b00011, 0b1100, "into"),              // bumped from N+Ih → N+Ao
    (0b01010, 0b0011, "work"),              // bumped from W+Er → W+Iy
    (0b10101, 0b0001, "their"),             // bumped from Dh+Eh → Dh+Ah
    (0b10101, 0b0010, "other"),             // bumped from Dh+Ah → Dh+Ih
    (0b10100, 0b0110, "great"),             // G+EY
    (0b00100, 0b1001, "keep"),              // bumped from K+Iy → K+Ay
    (0b00110, 0b0101, "long"),              // bumped from L+Ao → L+Aa
    (0b11000, 0b0011, "big"),               // bumped from B+Ih → B+Iy
    (0b00100, 0b0111, "kind"),              // bumped from K+Ay → K+Er
    (0b10001, 0b0101, "dad"),               // bumped from D+Ae → D+Aa
    (0b01100, 0b0011, "three"),             // TH+IY
    (0b01000, 0b1010, "own"),               // bumped from N+Ow → P+Ow
    (0b00101, 0b0110, "same"),              // bumped from S+Ey → R+Ey
    (0b01001, 0b0100, "next"),              // bumped from N+Eh → F+Eh
    (0b10110, 0b0100, "care"),              // bumped from K+Eh → L+mod+Eh
    (0b00000, 0b1110, "looking"),           // bumped from L+Uh → -+Uh
    (0b01000, 0b1100, "morning"),           // bumped from M+Ao → P+Ao
    (0b10000, 0b1100, "saw"),               // bumped from S+Ao → -+mod+Ao
    (0b00000, 0b1011, "move"),              // bumped from M+Uw → -+Uw
    (0b10000, 0b1010, "most"),              // bumped from M+Ow → -+mod+Ow
    (0b10010, 0b0101, "start"),             // bumped from S+Aa → Z+Aa
    (0b01110, 0b0000, "police"),            // bumped from P+Ah → Ng+-
    (0b11010, 0b0000, "stuff"),             // bumped from S+Ah → W+mod+-
    (0b11100, 0b0000, "under"),             // bumped from N+Ah → Th+mod+-
    (0b10100, 0b0101, "got"),               // G+AA
    (0b10101, 0b0100, "them"),              // DH+EH
    (0b00111, 0b1000, "had"),               // HH+AE
    (0b00000, 0b1101, "our"),               // -+AW
    (0b00001, 0b1011, "too"),               // T+UW
    (0b01010, 0b0110, "way"),               // W+EY
    (0b10011, 0b0110, "make"),              // M+EY
    (0b00010, 0b1011, "said"),              // bumped from S+Eh → S+Uw
    (0b00010, 0b1110, "sorry"),             // bumped from S+Aa → S+Uh
    (0b00011, 0b1001, "anything"),          // bumped from N+Eh → N+Ay
    (0b00011, 0b0111, "nothing"),           // bumped from N+Ah → N+Er
    (0b01001, 0b1000, "after"),             // F+AE
    (0b11001, 0b0000, "everything"),        // bumped from V+Eh → V+-
    (0b00001, 0b1110, "told"),              // bumped from T+Ow → T+Uh
    (0b00110, 0b1100, "always"),            // L+AO
    (0b00110, 0b1010, "leave"),             // bumped from L+Iy → L+Ow
    (0b01001, 0b0011, "feel"),              // F+IY
    (0b01000, 0b1001, "nice"),              // bumped from N+Ay → P+Ay
    (0b11000, 0b0110, "believe"),           // bumped from B+Ih → B+Ey
    (0b00111, 0b0011, "house"),             // bumped from H+Aw → H+Iy
    (0b01011, 0b0001, "understand"),        // bumped from N+Ah → Sh+Ah
    (0b01110, 0b0001, "son"),               // bumped from S+Ah → Ng+Ah
    (0b10000, 0b1001, "try"),               // bumped from T+Ay → -+mod+Ay
    (0b10001, 0b1010, "dead"),              // bumped from D+Eh → D+Ow
    (0b11010, 0b0001, "together"),          // bumped from T+Ah → W+mod+Ah
    (0b01011, 0b0010, "without"),           // bumped from W+Ih → Sh+Ih
    (0b10001, 0b1100, "already"),           // bumped from L+Ao → D+Ao
    (0b01110, 0b0010, "miss"),              // bumped from M+Ih → Ng+Ih
    (0b10110, 0b1000, "actually"),          // bumped from K+Ae → L+mod+Ae
    (0b01000, 0b0111, "heard"),             // bumped from H+Er → P+Er
    (0b11100, 0b0001, "once"),              // bumped from W+Ah → Th+mod+Ah
    (0b00101, 0b0101, "ready"),             // bumped from R+Eh → R+Aa
    (0b10010, 0b1100, "called"),            // bumped from K+Ao → Z+Ao
    (0b10010, 0b1010, "hold"),              // bumped from H+Ow → Z+Ow
    (0b01100, 0b0110, "haven"),             // bumped from H+Ey → Th+Ey
    (0b11010, 0b0010, "since"),             // bumped from S+Ih → W+mod+Ih
    (0b11100, 0b0010, "bring"),             // bumped from B+Ih → Th+mod+Ih
    (0b10000, 0b0111, "turn"),              // bumped from T+Er → -+mod+Er
    (0b10110, 0b0011, "eat"),               // bumped from T+Iy → L+mod+Iy
    (0b01101, 0b0000, "minute"),            // bumped from M+Ih → Ch+-
    (0b10111, 0b0000, "kid"),               // bumped from K+Ih → H+mod+-
    (0b01010, 0b0101, "want"),              // W+AA
    (0b10100, 0b1010, "going"),             // G+OW
    (0b00101, 0b1100, "or"),                // R+AO
    (0b00111, 0b0110, "hey"),               // HH+EY
    (0b00100, 0b1110, "could"),             // K+UH
    (0b10101, 0b0011, "these"),             // DH+IY
    (0b00010, 0b1101, "still"),             // bumped from S+Ih → S+Aw
    (0b00110, 0b1001, "life"),              // L+AY
    (0b10101, 0b1000, "than"),              // DH+AE
    (0b10011, 0b0101, "money"),             // bumped from M+Ah → M+Aa
    (0b11001, 0b0001, "ever"),              // bumped from V+Eh → V+Ah
    (0b00110, 0b0111, "old"),               // bumped from L+Ow → L+Er
    (0b00100, 0b1011, "coming"),            // bumped from K+Ah → K+Uw
    (0b11001, 0b0010, "every"),             // bumped from V+Eh → V+Ih
    (0b11000, 0b0101, "being"),             // bumped from B+Iy → B+Aa
    (0b00001, 0b1101, "today"),             // bumped from T+Ah → T+Aw
    (0b10100, 0b1100, "getting"),           // bumped from G+Eh → G+Ao
    (0b01011, 0b0100, "went"),              // bumped from W+Eh → Sh+Eh
    (0b01001, 0b0110, "friend"),            // bumped from F+Eh → F+Ey
    (0b10001, 0b1001, "trying"),            // bumped from T+Ay → D+Ay
    (0b10010, 0b1001, "live"),              // bumped from L+Ay → Z+Ay
    (0b01110, 0b0100, "head"),              // bumped from H+Eh → Ng+Eh
    (0b10001, 0b0111, "idea"),              // bumped from D+Ay → D+Er
    (0b01101, 0b0001, "such"),              // bumped from S+Ah → Ch+Ah
    (0b11010, 0b0100, "men"),               // bumped from M+Eh → W+mod+Eh
    (0b00101, 0b1010, "whole"),             // bumped from H+Ow → R+Ow
    (0b10111, 0b0001, "tomorrow"),          // bumped from T+Ah → H+mod+Ah
    (0b01100, 0b0101, "wanna"),             // bumped from W+Aa → Th+Aa
    (0b11100, 0b0100, "end"),               // bumped from N+Eh → Th+mod+Eh
    (0b10110, 0b0110, "saying"),            // bumped from S+Ey → L+mod+Ey
    (0b01101, 0b0010, "killed"),            // bumped from K+Ih → Ch+Ih
    (0b10111, 0b0010, "excuse"),            // bumped from K+Ih → H+mod+Ih
    (0b10010, 0b0111, "worry"),             // bumped from W+Er → Z+Er
    (0b10011, 0b1100, "more"),              // M+AO
    (0b11001, 0b0100, "very"),              // V+EH
    (0b01010, 0b1010, "wait"),              // bumped from W+Ey → W+Ow
    (0b01010, 0b1100, "won"),               // bumped from W+Ah → W+Ao
    (0b01100, 0b1100, "thought"),           // TH+AO
    (0b00011, 0b1011, "night"),             // bumped from N+Ay → N+Uw
    (0b01000, 0b1110, "put"),               // P+UH
    (0b00011, 0b1110, "new"),               // bumped from N+Uw → N+Uh
    (0b10100, 0b1001, "guys"),              // G+AY
    (0b01001, 0b0101, "father"),            // F+AA
    (0b10011, 0b1010, "made"),              // bumped from M+Ey → M+Ow
    (0b10100, 0b0111, "girl"),              // G+ER
    (0b00100, 0b1101, "ok"),                // bumped from K+Ow → K+Aw
    (0b00111, 0b0101, "happened"),          // bumped from H+Ae → H+Aa
    (0b11000, 0b1010, "bad"),               // bumped from B+Ae → B+Ow
    (0b10000, 0b1110, "woman"),             // bumped from W+Uh → -+mod+Uh
    (0b11000, 0b1100, "best"),              // bumped from B+Eh → B+Ao
    (0b01011, 0b1000, "shit"),              // bumped from Sh+Ih → Sh+Ae
    (0b01000, 0b1011, "knew"),              // bumped from N+Uw → P+Uw
    (0b01110, 0b1000, "happy"),             // bumped from H+Ae → Ng+Ae
    (0b00101, 0b1001, "while"),             // bumped from W+Ay → R+Ay
    (0b11010, 0b1000, "matter"),            // bumped from M+Ae → W+mod+Ae
    (0b00101, 0b0111, "run"),               // bumped from R+Ah → R+Er
    (0b10110, 0b0101, "hard"),              // bumped from H+Aa → L+mod+Aa
    (0b10000, 0b1011, "school"),            // bumped from S+Uw → -+mod+Uw
    (0b01101, 0b0100, "friends"),           // bumped from F+Eh → Ch+Eh
    (0b01100, 0b1010, "open"),              // bumped from P+Ow → Th+Ow
    (0b10111, 0b0100, "anyone"),            // bumped from N+Eh → H+mod+Eh
    (0b10101, 0b0110, "face"),              // bumped from F+Ey → Dh+Ey
    (0b11100, 0b1000, "hand"),              // bumped from H+Ae → Th+mod+Ae
    (0b11011, 0b0000, "drink"),             // bumped from D+Ih → Zh+-
    (0b11110, 0b0000, "its"),               // bumped from T+Ih → Ng+mod+-
    (0b01011, 0b0011, "whatever"),          // bumped from W+Ah → Sh+Iy
    (0b01110, 0b0011, "hit"),               // bumped from H+Ih → Ng+Iy
    (0b11010, 0b0011, "minutes"),           // bumped from M+Ih → W+mod+Iy
    (0b11100, 0b0011, "deal"),              // bumped from D+Iy → Th+mod+Iy
    (0b00011, 0b1101, "now"),               // N+AW
    (0b01010, 0b1001, "why"),               // W+AY
    (0b00110, 0b1110, "look"),              // L+UH
    (0b01010, 0b0111, "were"),              // W+ER
    (0b11000, 0b1001, "by"),                // B+AY
    (0b01001, 0b1100, "off"),               // F+AO
    (0b11001, 0b0011, "even"),              // V+IY
    (0b10001, 0b1011, "doing"),             // D+UW
    (0b00111, 0b1010, "home"),              // HH+OW
    (0b00110, 0b1011, "lot"),               // bumped from L+Aa → L+Uw
    (0b10011, 0b1001, "may"),               // bumped from M+Ey → M+Ay
    (0b10011, 0b0111, "mother"),            // bumped from M+Ah → M+Er
    (0b00111, 0b1100, "hear"),              // bumped from H+Iy → H+Ao
    (0b01001, 0b1010, "family"),            // bumped from F+Ae → F+Ow
    (0b11000, 0b0111, "baby"),              // bumped from B+Ey → B+Er
    (0b10001, 0b1110, "door"),              // bumped from D+Ao → D+Uh
    (0b10110, 0b1100, "also"),              // bumped from L+Ao → L+mod+Ao
    (0b01000, 0b1101, "pretty"),            // bumped from P+Ih → P+Aw
    (0b10010, 0b1110, "took"),              // bumped from T+Uh → Z+Uh
    (0b01111, 0b0000, "yet"),               // bumped from Y+Eh → Y+-
    (0b01100, 0b1001, "wife"),              // bumped from W+Ay → Th+Ay
    (0b11011, 0b0001, "tonight"),           // bumped from T+Ah → Zh+Ah
    (0b11001, 0b1000, "everyone"),          // bumped from V+Eh → V+Ae
    (0b00000, 0b1111, "ah"),                // bumped from -+Aa → -+Oy
    (0b11110, 0b0001, "alone"),             // bumped from L+Ah → Ng+mod+Ah
    (0b10101, 0b0101, "problem"),           // bumped from P+Aa → Dh+Aa
    (0b10010, 0b1011, "few"),               // bumped from F+Uw → Z+Uw
    (0b10110, 0b1010, "hope"),              // bumped from H+Ow → L+mod+Ow
    (0b11011, 0b0010, "business"),          // bumped from B+Ih → Zh+Ih
    (0b01011, 0b0110, "case"),              // bumped from K+Ey → Sh+Ey
    (0b01101, 0b0011, "each"),              // CH+IY
    (0b01110, 0b0110, "later"),             // bumped from L+Ey → Ng+Ey
    (0b01101, 0b1000, "having"),            // bumped from H+Ae → Ch+Ae
    (0b11110, 0b0010, "sit"),               // bumped from S+Ih → Ng+mod+Ih
    (0b01100, 0b0111, "thinking"),          // bumped from Th+Ih → Th+Er
    (0b11010, 0b0110, "late"),              // bumped from L+Ey → W+mod+Ey
    (0b11100, 0b0110, "pay"),               // bumped from P+Ey → Th+mod+Ey
    (0b10111, 0b1000, "happen"),            // bumped from H+Ae → H+mod+Ae
    (0b10000, 0b1101, "different"),         // bumped from D+Ih → -+mod+Aw
    (0b10111, 0b0011, "means"),             // bumped from M+Iy → H+mod+Iy
    (0b11101, 0b0000, "inside"),            // bumped from N+Ih → Jh+-
    (0b00111, 0b0111, "her"),               // HH+ER
    (0b10100, 0b1110, "good"),              // G+UH
    (0b10001, 0b1101, "down"),              // D+AW
    (0b01001, 0b0111, "first"),             // F+ER
    (0b01001, 0b1001, "find"),              // F+AY
    (0b00010, 0b1111, "sir"),               // bumped from S+Er → S+Oy
    (0b10101, 0b1010, "those"),             // DH+OW
    (0b01111, 0b0010, "years"),             // Y+IH
    (0b00110, 0b1101, "left"),              // bumped from L+Eh → L+Aw
    (0b00001, 0b1111, "talking"),           // bumped from T+Ao → T+Oy
    (0b00111, 0b1001, "hi"),                // HH+AY
    (0b00101, 0b1011, "room"),              // R+UW
    (0b01111, 0b0001, "use"),               // bumped from Y+Uw → Y+Ah
    (0b10100, 0b1011, "guess"),             // bumped from G+Eh → G+Uw
    (0b10110, 0b1001, "myself"),            // bumped from M+Ay → L+mod+Ay
    (0b11101, 0b0001, "um"),                // bumped from M+Ah → Jh+Ah
    (0b00101, 0b1110, "looks"),             // bumped from L+Uh → R+Uh
    (0b10101, 0b1100, "lost"),              // bumped from L+Ao → Dh+Ao
    (0b01011, 0b0101, "wants"),             // bumped from W+Aa → Sh+Aa
    (0b11011, 0b0100, "says"),              // bumped from S+Eh → Zh+Eh
    (0b01110, 0b0101, "heart"),             // bumped from H+Aa → Ng+Aa
    (0b11010, 0b0101, "watch"),             // bumped from W+Aa → W+mod+Aa
    (0b11100, 0b0101, "probably"),          // bumped from P+Aa → Th+mod+Aa
    (0b11110, 0b0100, "second"),            // bumped from S+Eh → Ng+mod+Eh
    (0b10110, 0b0111, "working"),           // bumped from W+Er → L+mod+Er
    (0b11101, 0b0010, "kids"),              // bumped from K+Ih → Jh+Ih
    (0b01101, 0b0110, "crazy"),             // bumped from K+Ey → Ch+Ey
    (0b11001, 0b0110, "everybody"),         // bumped from V+Eh → V+Ey
    (0b10111, 0b0110, "gave"),              // bumped from G+Ey → H+mod+Ey
    (0b10010, 0b1101, "eyes"),              // bumped from Z+Ay → Z+Aw
    (0b01111, 0b0100, "yes"),               // Y+EH
    (0b01010, 0b1110, "would"),             // W+UH
    (0b01010, 0b1011, "away"),              // bumped from W+Ah → W+Uw
    (0b01100, 0b1011, "through"),           // TH+UW
    (0b00100, 0b1111, "course"),            // bumped from K+Ao → K+Oy
    (0b10011, 0b1011, "might"),             // bumped from M+Ay → M+Uw
    (0b01011, 0b1010, "show"),              // SH+OW
    (0b10011, 0b1110, "many"),              // bumped from M+Eh → M+Uh
    (0b11000, 0b1011, "both"),              // bumped from B+Ow → B+Uw
    (0b11000, 0b1110, "brother"),           // bumped from B+Ah → B+Uh
    (0b10100, 0b1101, "gone"),              // bumped from G+Ao → G+Aw
    (0b10101, 0b1001, "die"),               // bumped from D+Ay → Dh+Ay
    (0b01101, 0b0101, "doctor"),            // bumped from D+Aa → Ch+Aa
    (0b01011, 0b1100, "water"),             // bumped from W+Ao → Sh+Ao
    (0b10101, 0b0111, "person"),            // bumped from P+Er → Dh+Er
    (0b11001, 0b0101, "part"),              // bumped from P+Aa → V+Aa
    (0b11101, 0b0100, "death"),             // bumped from D+Eh → Jh+Eh
    (0b11011, 0b1000, "damn"),              // bumped from D+Ae → Zh+Ae
    (0b10111, 0b0101, "far"),               // bumped from F+Aa → H+mod+Aa
    (0b01110, 0b1010, "knows"),             // bumped from N+Ow → Ng+Ow
    (0b00101, 0b1101, "aren"),              // bumped from R+Aa → R+Aw
    (0b11110, 0b1000, "hands"),             // bumped from H+Ae → Ng+mod+Ae
    (0b01100, 0b1110, "somebody"),          // bumped from S+Ah → Th+Uh
    (0b01110, 0b1100, "afraid"),            // bumped from F+Ah → Ng+Ao
    (0b11011, 0b0011, "sleep"),             // bumped from S+Iy → Zh+Iy
    (0b11010, 0b1010, "dear"),              // bumped from D+Ih → W+mod+Ow
    (0b11010, 0b1100, "four"),              // bumped from F+Ao → W+mod+Ao
    (0b11100, 0b1010, "close"),             // bumped from K+Ow → Th+mod+Ow
    (0b11100, 0b1100, "fun"),               // bumped from F+Ah → Th+mod+Ao
    (0b11110, 0b0011, "against"),           // bumped from G+Ah → Ng+mod+Iy
    (0b01111, 0b1000, "yeah"),              // Y+AE
    (0b00111, 0b1011, "who"),               // HH+UW
    (0b11001, 0b1010, "over"),              // V+OW
    (0b00011, 0b1111, "around"),            // bumped from N+Er → N+Oy
    (0b01010, 0b1101, "which"),             // bumped from W+Ih → W+Aw
    (0b10011, 0b1101, "mind"),              // bumped from M+Ay → M+Aw
    (0b00111, 0b1110, "hell"),              // bumped from H+Eh → H+Uh
    (0b01001, 0b1011, "fuck"),              // bumped from F+Ah → F+Uw
    (0b11000, 0b1101, "bit"),               // bumped from B+Ih → B+Aw
    (0b01001, 0b1110, "phone"),             // bumped from F+Ow → F+Uh
    (0b01000, 0b1111, "play"),              // bumped from P+Ey → P+Oy
    (0b10110, 0b1011, "true"),              // bumped from T+Uw → L+mod+Uw
    (0b01111, 0b0011, "year"),              // bumped from Y+Ih → Y+Iy
    (0b01011, 0b0111, "forget"),            // bumped from F+Er → Sh+Er
    (0b01101, 0b1010, "change"),            // bumped from Ch+Ey → Ch+Ow
    (0b01011, 0b1001, "five"),              // bumped from F+Ay → Sh+Ay
    (0b11011, 0b0110, "taking"),            // bumped from T+Ey → Zh+Ey
    (0b01110, 0b1001, "times"),             // bumped from T+Ay → Ng+Ay
    (0b01101, 0b1100, "check"),             // bumped from Ch+Eh → Ch+Ao
    (0b10000, 0b1111, "point"),             // bumped from P+Oy → -+mod+Oy
    (0b11110, 0b0110, "makes"),             // bumped from M+Ey → Ng+mod+Ey
    (0b11101, 0b1000, "asked"),             // bumped from S+Ae → Jh+Ae
    (0b11010, 0b1001, "mine"),              // bumped from M+Ay → W+mod+Ay
    (0b11100, 0b1001, "quite"),             // bumped from K+Ay → Th+mod+Ay
    (0b01110, 0b0111, "word"),              // bumped from W+Er → Ng+Er
    (0b01100, 0b1101, "comes"),             // bumped from K+Ah → Th+Aw
    (0b11001, 0b1100, "important"),         // bumped from M+Ih → V+Ao
    (0b11111, 0b0000, "set"),               // bumped from S+Eh → Y+mod+-
    (0b10111, 0b1100, "story"),             // bumped from S+Ao → H+mod+Ao
    (0b10110, 0b1110, "number"),            // bumped from N+Ah → L+mod+Uh
    (0b11101, 0b0011, "least"),             // bumped from L+Iy → Jh+Iy
    (0b11010, 0b0111, "hurt"),              // bumped from H+Er → W+mod+Er
    (0b10111, 0b1010, "wish"),              // bumped from W+Ih → H+mod+Ow
    (0b11100, 0b0111, "husband"),           // bumped from H+Ah → Th+mod+Er
    (0b00111, 0b1101, "how"),               // HH+AW
    (0b00110, 0b1111, "listen"),            // bumped from L+Ih → L+Oy
    (0b01001, 0b1101, "found"),             // F+AW
    (0b10001, 0b1111, "days"),              // bumped from D+Ey → D+Oy
    (0b11111, 0b0001, "ago"),               // bumped from G+Ah → Y+mod+Ah
    (0b10101, 0b1011, "soon"),              // bumped from S+Uw → Dh+Uw
    (0b01111, 0b0110, "young"),             // bumped from Y+Ah → Y+Ey
    (0b11111, 0b0010, "exactly"),           // bumped from G+Ih → Y+mod+Ih
    (0b10010, 0b1111, "easy"),              // bumped from Z+Iy → Z+Oy
    (0b11101, 0b0110, "making"),            // bumped from M+Ey → Jh+Ey
    (0b11011, 0b0101, "body"),              // bumped from B+Aa → Zh+Aa
    (0b01101, 0b1001, "chance"),            // bumped from Ch+Ae → Ch+Ay
    (0b11110, 0b0101, "party"),             // bumped from P+Aa → Ng+mod+Aa
    (0b11001, 0b1001, "fight"),             // bumped from F+Ay → V+Ay
    (0b01101, 0b0111, "girls"),             // bumped from G+Er → Ch+Er
    (0b10101, 0b1110, "married"),           // bumped from M+Eh → Dh+Uh
    (0b10111, 0b1001, "fire"),              // bumped from F+Ay → H+mod+Ay
    (0b10110, 0b1101, "game"),              // bumped from G+Ey → L+mod+Aw
    (0b11001, 0b0111, "mr"),                // bumped from M+Ih → V+Er
    (0b10111, 0b0111, "read"),              // bumped from R+Eh → H+mod+Er
    (0b01011, 0b1110, "should"),            // SH+UH
    (0b11101, 0b0101, "job"),               // JH+AA
    (0b10100, 0b1111, "gotta"),             // bumped from G+Aa → G+Oy
    (0b01011, 0b1011, "music"),             // bumped from M+Uw → Sh+Uw
    (0b01110, 0b1011, "beautiful"),         // bumped from B+Uw → Ng+Uw
    (0b11111, 0b0100, "anyway"),            // bumped from N+Eh → Y+mod+Eh
    (0b11011, 0b1100, "daughter"),          // bumped from D+Ao → Zh+Ao
    (0b11011, 0b1010, "moment"),            // bumped from M+Ow → Zh+Ow
    (0b00101, 0b1111, "rest"),              // bumped from R+Eh → R+Oy
    (0b11110, 0b1010, "nobody"),            // bumped from N+Ow → Ng+mod+Ow
    (0b10101, 0b1101, "though"),            // bumped from Dh+Ow → Dh+Aw
    (0b01110, 0b1110, "cut"),               // bumped from K+Ah → Ng+Uh
    (0b01111, 0b0101, "started"),           // bumped from S+Aa → Y+Aa
    (0b11010, 0b1011, "sister"),            // bumped from S+Ih → W+mod+Uw
    (0b11010, 0b1110, "supposed"),          // bumped from S+Ah → W+mod+Uh
    (0b11100, 0b1011, "between"),           // bumped from B+Ih → Th+mod+Uw
    (0b11100, 0b1110, "speak"),             // bumped from S+Iy → Th+mod+Uh
    (0b11110, 0b1100, "women"),             // bumped from W+Ih → Ng+mod+Ao
    (0b01010, 0b1111, "wanted"),            // bumped from W+Ao → W+Oy
    (0b10011, 0b1111, "mom"),               // bumped from M+Aa → M+Oy
    (0b11000, 0b1111, "boy"),               // B+OY
    (0b01011, 0b1101, "shut"),              // bumped from Sh+Ah → Sh+Aw
    (0b11111, 0b0011, "week"),              // bumped from W+Iy → Y+mod+Iy
    (0b01101, 0b1011, "children"),          // bumped from Ch+Ih → Ch+Uw
    (0b11011, 0b1001, "side"),              // bumped from S+Ay → Zh+Ay
    (0b11111, 0b1000, "stand"),             // bumped from S+Ae → Y+mod+Ae
    (0b01101, 0b1110, "child"),             // bumped from Ch+Ay → Ch+Uh
    (0b01111, 0b1010, "goes"),              // bumped from G+Ow → Y+Ow
    (0b01110, 0b1101, "hours"),             // bumped from Z+Aw → Ng+Aw
    (0b01100, 0b1111, "behind"),            // bumped from B+Ih → Th+Oy
    (0b01111, 0b1100, "almost"),            // bumped from L+Ao → Y+Ao
    (0b11001, 0b1011, "truth"),             // bumped from T+Uw → V+Uw
    (0b11001, 0b1110, "blood"),             // bumped from B+Ah → V+Uh
    (0b11010, 0b1101, "able"),              // bumped from B+Ey → W+mod+Aw
    (0b11100, 0b1101, "lady"),              // bumped from L+Ey → Th+mod+Aw
    (0b11101, 0b1010, "anymore"),           // bumped from N+Eh → Jh+Ow
    (0b11101, 0b1100, "playing"),           // bumped from P+Ey → Jh+Ao
    (0b11110, 0b1001, "gets"),              // bumped from G+Eh → Ng+mod+Ay
    (0b10111, 0b1011, "reason"),            // bumped from R+Iy → H+mod+Uw
    (0b10111, 0b1110, "trouble"),           // bumped from T+Ah → H+mod+Uh
    (0b11011, 0b0111, "break"),             // bumped from B+Ey → Zh+Er
    (0b11110, 0b0111, "city"),              // bumped from S+Ih → Ng+mod+Er
    (0b01111, 0b0111, "yourself"),          // Y+ER
    (0b00111, 0b1111, "huh"),               // bumped from H+Ah → H+Oy
    (0b01001, 0b1111, "fucking"),           // bumped from F+Ah → F+Oy
    (0b11111, 0b0110, "waiting"),           // bumped from W+Ey → Y+mod+Ey
    (0b01101, 0b1101, "walk"),              // bumped from W+Ao → Ch+Aw
    (0b11001, 0b1101, "town"),              // bumped from T+Aw → V+Aw
    (0b01111, 0b1001, "trust"),             // bumped from T+Ah → Y+Ay
    (0b11101, 0b1001, "met"),               // bumped from M+Eh → Jh+Ay
    (0b10110, 0b1111, "office"),            // bumped from F+Ao → L+mod+Oy
    (0b10111, 0b1101, "question"),          // bumped from K+Eh → H+mod+Aw
    (0b11101, 0b0111, "brought"),           // bumped from B+Ao → Jh+Er
    (0b11111, 0b0101, "shot"),              // bumped from Sh+Aa → Y+mod+Aa
    (0b10101, 0b1111, "welcome"),           // bumped from W+Eh → Dh+Oy
    (0b11011, 0b1011, "couple"),            // bumped from K+Ah → Zh+Uw
    (0b11011, 0b1110, "half"),              // bumped from H+Ae → Zh+Uh
    (0b11110, 0b1011, "died"),              // bumped from D+Ay → Ng+mod+Uw
    (0b11110, 0b1110, "free"),              // bumped from F+Iy → Ng+mod+Uh
    (0b01111, 0b1011, "used"),              // Y+UW
    (0b01011, 0b1111, "shall"),             // bumped from Sh+Ae → Sh+Oy
    (0b11111, 0b1100, "war"),               // bumped from W+Ao → Y+mod+Ao
    (0b01111, 0b1110, "yours"),             // Y+UH
    (0b11011, 0b1101, "wow"),               // bumped from W+Aw → Zh+Aw
    (0b11101, 0b1011, "cool"),              // bumped from K+Uw → Jh+Uw
    (0b01110, 0b1111, "either"),            // bumped from Dh+Iy → Ng+Oy
    (0b11010, 0b1111, "seems"),             // bumped from S+Iy → W+mod+Oy
    (0b11110, 0b1101, "power"),             // bumped from P+Aw → Ng+mod+Aw
    (0b11111, 0b1010, "whoa"),              // bumped from W+Ow → Y+mod+Ow
    (0b11100, 0b1111, "bye"),               // bumped from B+Ay → Th+mod+Oy
    (0b11101, 0b1110, "buy"),               // bumped from B+Ay → Jh+Uh
    (0b11111, 0b1001, "high"),              // bumped from H+Ay → Y+mod+Ay
    (0b01101, 0b1111, "telling"),           // bumped from T+Eh → Ch+Oy
    (0b01111, 0b1101, "honey"),             // bumped from H+Ah → Y+Aw
    (0b11001, 0b1111, "tried"),             // bumped from T+Ay → V+Oy
    (0b11101, 0b1101, "front"),             // bumped from F+Ah → Jh+Aw
    (0b10111, 0b1111, "team"),              // bumped from T+Iy → H+mod+Oy
    (0b11111, 0b0111, "answer"),            // bumped from N+Ae → Y+mod+Er
    (0b11011, 0b1111, "gun"),               // bumped from G+Ah → Zh+Oy
    (0b11110, 0b1111, "boys"),              // bumped from B+Oy → Ng+mod+Oy
    (0b11111, 0b1011, "line"),              // bumped from L+Ay → Y+mod+Uw
    (0b11111, 0b1110, "send"),              // bumped from S+Eh → Y+mod+Uh
    (0b01111, 0b1111, "news"),              // bumped from N+Uw → Y+Oy
    (0b11101, 0b1111, "stupid"),            // bumped from S+Uw → Jh+Oy
    (0b11111, 0b1101, "bed"),               // bumped from B+Eh → Y+mod+Aw
    (0b11111, 0b1111, "hurry"),             // bumped from H+Er → Y+mod+Oy
];
