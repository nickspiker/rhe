//! English number-form generators for the number-mode suffix feature.
//!
//! After a pure-integer cardinal commits in number mode, a left-hand
//! chord in the subsequent word-not-held state fires a *form* — the
//! chord's English-suffix meaning gets overridden with a number-form
//! transform that replaces the emitted digits with the spelled form.
//!
//! Currently only the ordinal form is implemented. The shape of this
//! module anticipates adding multiplier / group / prefix forms later
//! without churn — each form is a single function taking the digit
//! string and returning the spelled equivalent (or `None` if the
//! number is outside the supported range, in which case the caller
//! falls back to the normal English-suffix path).

/// Forms a number-mode commit can be transformed into. Each variant
/// maps to a specific left-hand chord and a generator function
/// below. Adding a form = one enum variant + one generator + one
/// entry in `chord_to_form` in the interpreter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Form {
    /// Spelled cardinal — "42" → "forty-two". Redundant with the
    /// mod-after-number spelled-digit gesture but faster via L-idx.
    SpelledCardinal,
    /// Ordinal — "42" → "forty-second".
    Ordinal,
    /// Multiplier — "1" → "once", "2" → "twice", "3" → "thrice",
    /// higher → "N times".
    Multiplier,
    /// Group / multiple — "2" → "pair", "3" → "triple", "4" →
    /// "quadruple". Rare beyond 10.
    Group,
    /// Fraction denominator — "2" → "half", "3" → "third", "4" →
    /// "quarter", higher → ordinal form ("fifth", "sixth", ...).
    Fraction,
    /// Greek/Latin prefix — "1" → "mono", "2" → "bi", "3" → "tri",
    /// "4" → "tetra", up to "deca" (10). Technical vocabulary.
    Prefix,
}

/// Apply a form to a digit string. Returns `None` if the number is
/// outside the form's supported range (callers should fall back to
/// the normal English-suffix path).
pub fn apply(form: Form, digits: &str) -> Option<String> {
    match form {
        Form::SpelledCardinal => spelled_cardinal(digits),
        Form::Ordinal => ordinal(digits),
        Form::Multiplier => multiplier(digits),
        Form::Group => group(digits),
        Form::Fraction => fraction(digits),
        Form::Prefix => prefix(digits),
    }
}

/// Spell `n` as an English ordinal. Returns `None` if the number is
/// larger than what this table covers (v1: 0-999 plus exact thousand
/// and million). Callers should fall back to the English suffix path
/// on `None`.
pub fn ordinal(digits: &str) -> Option<String> {
    let n: u64 = digits.parse().ok()?;
    spell_ordinal(n)
}

pub fn spelled_cardinal(digits: &str) -> Option<String> {
    let n: u64 = digits.parse().ok()?;
    spell_cardinal(n)
}

pub fn multiplier(digits: &str) -> Option<String> {
    let n: u64 = digits.parse().ok()?;
    match n {
        1 => Some("once".into()),
        2 => Some("twice".into()),
        3 => Some("thrice".into()),
        n if n < 100 => Some(format!("{} times", spell_under_hundred_cardinal(n))),
        _ => None,
    }
}

pub fn group(digits: &str) -> Option<String> {
    let n: u64 = digits.parse().ok()?;
    Some(
        match n {
            1 => "single",
            2 => "pair",
            3 => "triple",
            4 => "quadruple",
            5 => "quintuple",
            6 => "sextuple",
            7 => "septuple",
            8 => "octuple",
            9 => "nonuple",
            10 => "decuple",
            _ => return None,
        }
        .into(),
    )
}

pub fn fraction(digits: &str) -> Option<String> {
    let n: u64 = digits.parse().ok()?;
    match n {
        // "half" and "quarter" are the only fraction-specific words
        // in English. 1/3 = "third" but that's also the ordinal —
        // same output, chord-context disambiguates intent.
        2 => Some("half".into()),
        3 => Some("third".into()),
        4 => Some("quarter".into()),
        n if n >= 5 && n < 100 => spell_under_hundred_ordinal(n),
        _ => None,
    }
}

pub fn prefix(digits: &str) -> Option<String> {
    let n: u64 = digits.parse().ok()?;
    Some(
        match n {
            1 => "mono",
            2 => "bi",
            3 => "tri",
            4 => "tetra",
            5 => "penta",
            6 => "hexa",
            7 => "hepta",
            8 => "octa",
            9 => "nona",
            10 => "deca",
            _ => return None,
        }
        .into(),
    )
}

fn spell_cardinal(n: u64) -> Option<String> {
    if n < 100 {
        return Some(spell_under_hundred_cardinal(n));
    }
    if n < 1000 {
        let hundreds = n / 100;
        let rem = n % 100;
        let h = spell_under_hundred_cardinal(hundreds);
        if rem == 0 {
            return Some(format!("{} hundred", h));
        }
        return Some(format!("{} hundred {}", h, spell_under_hundred_cardinal(rem)));
    }
    match n {
        1_000 => Some("one thousand".into()),
        1_000_000 => Some("one million".into()),
        _ => None,
    }
}

fn spell_ordinal(n: u64) -> Option<String> {
    // Exact-match special cases (million/billion) go first so we don't
    // trip over the "one million" prefix in the general speller.
    match n {
        0 => return Some("zeroth".into()),
        1_000 => return Some("thousandth".into()),
        1_000_000 => return Some("millionth".into()),
        1_000_000_000 => return Some("billionth".into()),
        _ => {}
    }
    if n < 100 {
        return spell_under_hundred_ordinal(n);
    }
    if n < 1_000 {
        return spell_under_thousand_ordinal(n);
    }
    // v1 doesn't compose ordinals beyond 999; fallback to English -ing.
    None
}

fn spell_under_twenty_ordinal(n: u64) -> &'static str {
    match n {
        1 => "first",
        2 => "second",
        3 => "third",
        4 => "fourth",
        5 => "fifth",
        6 => "sixth",
        7 => "seventh",
        8 => "eighth",
        9 => "ninth",
        10 => "tenth",
        11 => "eleventh",
        12 => "twelfth",
        13 => "thirteenth",
        14 => "fourteenth",
        15 => "fifteenth",
        16 => "sixteenth",
        17 => "seventeenth",
        18 => "eighteenth",
        19 => "nineteenth",
        _ => unreachable!(),
    }
}

fn decade_word(tens: u64) -> &'static str {
    match tens {
        2 => "twenty",
        3 => "thirty",
        4 => "forty",
        5 => "fifty",
        6 => "sixty",
        7 => "seventy",
        8 => "eighty",
        9 => "ninety",
        _ => unreachable!(),
    }
}

fn decade_ordinal(tens: u64) -> &'static str {
    match tens {
        2 => "twentieth",
        3 => "thirtieth",
        4 => "fortieth",
        5 => "fiftieth",
        6 => "sixtieth",
        7 => "seventieth",
        8 => "eightieth",
        9 => "ninetieth",
        _ => unreachable!(),
    }
}

fn spell_under_hundred_ordinal(n: u64) -> Option<String> {
    if n < 20 {
        return Some(spell_under_twenty_ordinal(n).into());
    }
    let tens = n / 10;
    let units = n % 10;
    if units == 0 {
        return Some(decade_ordinal(tens).into());
    }
    // "twenty-first", "forty-second", etc.
    Some(format!("{}-{}", decade_word(tens), spell_under_twenty_ordinal(units)))
}

fn spell_under_hundred_cardinal(n: u64) -> String {
    if n < 20 {
        return match n {
            0 => "zero",
            1 => "one",
            2 => "two",
            3 => "three",
            4 => "four",
            5 => "five",
            6 => "six",
            7 => "seven",
            8 => "eight",
            9 => "nine",
            10 => "ten",
            11 => "eleven",
            12 => "twelve",
            13 => "thirteen",
            14 => "fourteen",
            15 => "fifteen",
            16 => "sixteen",
            17 => "seventeen",
            18 => "eighteen",
            19 => "nineteen",
            _ => unreachable!(),
        }
        .into();
    }
    let tens = n / 10;
    let units = n % 10;
    if units == 0 {
        return decade_word(tens).into();
    }
    format!("{}-{}", decade_word(tens), spell_under_hundred_cardinal(units))
}

fn spell_under_thousand_ordinal(n: u64) -> Option<String> {
    // 100-999. Hundreds digit + (cardinal "hundred") + (remainder
    // spelled as ordinal). "two hundred first", "five hundred
    // twenty-third", "seven hundredth".
    let hundreds = n / 100;
    let remainder = n % 100;
    let hundreds_word = spell_under_hundred_cardinal(hundreds);
    if remainder == 0 {
        return Some(format!("{} hundredth", hundreds_word));
    }
    let rem_ord = spell_under_hundred_ordinal(remainder)?;
    Some(format!("{} hundred {}", hundreds_word, rem_ord))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinal_sub_twenty() {
        assert_eq!(ordinal("1").as_deref(), Some("first"));
        assert_eq!(ordinal("2").as_deref(), Some("second"));
        assert_eq!(ordinal("3").as_deref(), Some("third"));
        assert_eq!(ordinal("5").as_deref(), Some("fifth"));
        assert_eq!(ordinal("8").as_deref(), Some("eighth"));
        assert_eq!(ordinal("9").as_deref(), Some("ninth"));
        assert_eq!(ordinal("12").as_deref(), Some("twelfth"));
        assert_eq!(ordinal("19").as_deref(), Some("nineteenth"));
    }

    #[test]
    fn ordinal_decades() {
        assert_eq!(ordinal("20").as_deref(), Some("twentieth"));
        assert_eq!(ordinal("50").as_deref(), Some("fiftieth"));
        assert_eq!(ordinal("90").as_deref(), Some("ninetieth"));
    }

    #[test]
    fn ordinal_compound() {
        assert_eq!(ordinal("21").as_deref(), Some("twenty-first"));
        assert_eq!(ordinal("42").as_deref(), Some("forty-second"));
        assert_eq!(ordinal("55").as_deref(), Some("fifty-fifth"));
        assert_eq!(ordinal("99").as_deref(), Some("ninety-ninth"));
    }

    #[test]
    fn ordinal_hundreds() {
        assert_eq!(ordinal("100").as_deref(), Some("one hundredth"));
        assert_eq!(ordinal("200").as_deref(), Some("two hundredth"));
        assert_eq!(ordinal("101").as_deref(), Some("one hundred first"));
        assert_eq!(ordinal("523").as_deref(), Some("five hundred twenty-third"));
    }

    #[test]
    fn ordinal_thousand_million() {
        assert_eq!(ordinal("1000").as_deref(), Some("thousandth"));
        assert_eq!(ordinal("1000000").as_deref(), Some("millionth"));
    }

    #[test]
    fn ordinal_zero() {
        assert_eq!(ordinal("0").as_deref(), Some("zeroth"));
    }

    #[test]
    fn ordinal_out_of_range_returns_none() {
        // 1921, 2026, arbitrary thousands — v1 fallback.
        assert!(ordinal("1921").is_none());
        assert!(ordinal("2026").is_none());
    }

    #[test]
    fn ordinal_invalid_input() {
        assert!(ordinal("").is_none());
        assert!(ordinal("abc").is_none());
        assert!(ordinal("3.14").is_none());
    }

    #[test]
    fn spelled_cardinal_basics() {
        assert_eq!(spelled_cardinal("0").as_deref(), Some("zero"));
        assert_eq!(spelled_cardinal("1").as_deref(), Some("one"));
        assert_eq!(spelled_cardinal("42").as_deref(), Some("forty-two"));
        assert_eq!(spelled_cardinal("100").as_deref(), Some("one hundred"));
        assert_eq!(spelled_cardinal("523").as_deref(), Some("five hundred twenty-three"));
        assert_eq!(spelled_cardinal("1000").as_deref(), Some("one thousand"));
    }

    #[test]
    fn multiplier_cases() {
        assert_eq!(multiplier("1").as_deref(), Some("once"));
        assert_eq!(multiplier("2").as_deref(), Some("twice"));
        assert_eq!(multiplier("3").as_deref(), Some("thrice"));
        assert_eq!(multiplier("4").as_deref(), Some("four times"));
        assert_eq!(multiplier("42").as_deref(), Some("forty-two times"));
    }

    #[test]
    fn group_cases() {
        assert_eq!(group("2").as_deref(), Some("pair"));
        assert_eq!(group("3").as_deref(), Some("triple"));
        assert_eq!(group("4").as_deref(), Some("quadruple"));
        assert_eq!(group("10").as_deref(), Some("decuple"));
        assert!(group("11").is_none());
    }

    #[test]
    fn fraction_cases() {
        assert_eq!(fraction("2").as_deref(), Some("half"));
        assert_eq!(fraction("3").as_deref(), Some("third"));
        assert_eq!(fraction("4").as_deref(), Some("quarter"));
        assert_eq!(fraction("5").as_deref(), Some("fifth"));
        assert_eq!(fraction("20").as_deref(), Some("twentieth"));
    }

    #[test]
    fn prefix_cases() {
        assert_eq!(prefix("1").as_deref(), Some("mono"));
        assert_eq!(prefix("2").as_deref(), Some("bi"));
        assert_eq!(prefix("3").as_deref(), Some("tri"));
        assert_eq!(prefix("5").as_deref(), Some("penta"));
        assert_eq!(prefix("10").as_deref(), Some("deca"));
        assert!(prefix("11").is_none());
    }

    #[test]
    fn apply_dispatcher() {
        assert_eq!(apply(Form::Ordinal, "3").as_deref(), Some("third"));
        assert_eq!(apply(Form::Multiplier, "2").as_deref(), Some("twice"));
        assert_eq!(apply(Form::Group, "3").as_deref(), Some("triple"));
        assert_eq!(apply(Form::Fraction, "2").as_deref(), Some("half"));
        assert_eq!(apply(Form::Prefix, "4").as_deref(), Some("tetra"));
        assert_eq!(apply(Form::SpelledCardinal, "42").as_deref(), Some("forty-two"));
    }
}
