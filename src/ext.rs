use crate::{ file::PostFile, target::Target };
use itertools::Itertools;
use std::collections::HashSet;

pub fn list(files: HashSet<PostFile>, target: &Target) {
    let mut extensions = HashSet::new();
    let mut no_extension: u32 = 0;

    for file in files {
        if let Some(extension) = file.to_extension(target) {
            extensions.insert(extension);
        } else {
            no_extension += 1;
        }
    }

    if !extensions.is_empty() {
        eprintln!("{}", extensions.into_iter().sorted().join(","));
    }

    if no_extension != 0 {
        eprintln!("{no_extension} files do not have an extension");
    }
}
