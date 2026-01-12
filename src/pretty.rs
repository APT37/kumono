use num_format::{ Locale, ToFormattedString };

pub fn n_fmt(n: u64) -> String {
    n.to_formatted_string(&Locale::en)
}

pub fn with_noun(count: usize, noun: &str) -> String {
    match count {
        0 => format!("no {noun}s"),
        1 => format!("1 {noun}"),
        n => format!("{number} {noun}s", number = n_fmt(n as u64)),
    }
}
