use num_format::{ Locale, ToFormattedString };
use std::fmt::Write;

pub fn n_fmt(n: u64) -> String {
    n.to_formatted_string(&Locale::en)
}

pub fn with_word(n: u64, word: &str) -> String {
    let number = n_fmt(n);

    let mut buf = String::with_capacity(number.len() + 3 + word.len());

    let _ = match n {
        0 => write!(buf, "no {word}s"),
        1 => write!(buf, "1 {word}"),
        _ => write!(buf, "{number} {word}s"),
    };

    buf
}
