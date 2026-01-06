use crate::{ file::PostFile, target::Target };
use std::collections::HashSet;

pub fn list(files: HashSet<PostFile>, target: &Target) {
    let mut extensions = HashSet::new();
    let mut no_ext = 0;

    for file in files {
        if let Some(ext) = file.to_extension(target) {
            extensions.insert(ext.to_lowercase());
        } else {
            no_ext += 1;
        }
    }

    if !extensions.is_empty() {
        eprintln!("{exts}", exts = extensions.into_iter().collect::<Vec<_>>().join(","));
    }

    if no_ext > 0 {
        eprintln!("{no_ext} files do not have an extension");
    }
}
