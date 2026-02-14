use num_format::{ Locale, ToFormattedString };
use std::fmt::Write;

pub fn n_fmt(number: usize) -> String {
    number.to_formatted_string(&Locale::en)
}

pub fn with_word(number: usize, word: &str) -> String {
    let num = n_fmt(number);

    let mut buf = String::with_capacity(num.len() + 3 + word.len());

    let _ = match number {
        0 => write!(buf, "no {word}s"),
        1 => write!(buf, "1 {word}"),
        _ => write!(buf, "{num} {word}s"),
    };

    buf
}
