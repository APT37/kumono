use crate::{ file::PostFile, target::Target };
use itertools::Itertools;
use std::{ collections::{ HashMap, HashSet }, fmt::{ Display, Formatter, Result } };

#[derive(Default)]
pub struct ExtensionList {
    extensions: HashSet<String>,
    without_extension: usize,
}

impl ExtensionList {
    pub fn new(files: &HashSet<PostFile>, target: &Target) -> Self {
        let mut ext_list = ExtensionList::default();

        for file in files {
            match file.to_extension(target) {
                Some(ext) => ext_list.add_ext(ext),
                None => ext_list.add_no_ext(),
            }
        }

        ext_list
    }

    fn add_ext(&mut self, extension: String) {
        self.extensions.insert(extension);
    }

    fn add_no_ext(&mut self) {
        self.without_extension += 1;
    }
}

impl Display for ExtensionList {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        if !self.extensions.is_empty() {
            write!(f, "{}", self.extensions.iter().sorted().join(","))?;
        }

        if self.without_extension != 0 {
            if !self.extensions.is_empty() {
                writeln!(f)?;
            }
            
            write!(f, "{} files do not have an extension", self.without_extension)?;
        }

        Ok(())
    }
}

pub fn count(files: &HashSet<PostFile>, target: &Target) -> HashMap<String, usize> {
    let mut files_by_type = HashMap::new();

    for file in files {
        *files_by_type
            .entry(file.to_extension(target).unwrap_or_else(|| "unknown".to_string()))
            .or_default() += 1;
    }

    files_by_type
}
