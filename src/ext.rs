use crate::file::PostFile;
use itertools::Itertools;
use std::{ collections::{ HashMap, HashSet }, fmt::{ Display, Formatter, Result }, sync::Arc };

#[derive(Default)]
pub struct ExtensionList {
    extensions: HashSet<String>,
    without_extension: usize,
}

impl ExtensionList {
    pub fn new(files: &HashSet<Arc<PostFile>>) -> Self {
        let mut ext_list = ExtensionList::default();

        for file in files {
            match file.get_ext() {
                Some(ext) => {
                    if !ext_list.extensions.contains("ext") {
                        ext_list.extensions.insert(ext.to_string());
                    }
                }
                None => {
                    ext_list.without_extension += 1;
                }
            }
        }

        ext_list
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

pub fn count(files: &HashSet<Arc<PostFile>>) -> HashMap<String, usize> {
    let mut files_by_type: HashMap<String, _> = HashMap::new();

    for file in files {
        *files_by_type
            .entry(file.get_ext().map_or_else(|| "none".to_string(), String::from))
            .or_default() += 1;
    }

    files_by_type
}
