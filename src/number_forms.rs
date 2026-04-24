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

/// Spell `n` as an English ordinal. Returns `None` if the number is
/// larger than what this table covers (v1: 0-999 plus exact thousand
/// and million). Callers should fall back to the English suffix path
/// on `None`.
pub fn ordinal(digits: &str) -> Option<String> {
    let n: u64 = digits.parse().ok()?;
    spell_ordinal(n)
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
}
