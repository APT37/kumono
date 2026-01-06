use num_format::{ Locale, ToFormattedString };

pub fn n_fmt(n: u64) -> String {
    n.to_formatted_string(&Locale::en)
}

pub fn files(files: usize) -> String {
    anything(files, "file", "files")
}

pub fn posts(posts: usize) -> String {
    anything(posts, "post", "posts")
}

pub fn anything(count: usize, singular: &str, plural: &str) -> String {
    match count {
        0 => format!("no {plural}"),
        1 => format!("1 {singular}"),
        n => format!("{number} {plural}", number = n_fmt(n as u64)),
    }
}
