#[derive(Debug, Clone, Default)]
pub struct FileName {
    pub basename: String,
    pub extension: Option<String>,
}

impl FileName {
    pub fn new(basename: String, extension: Option<String>) -> FileName {
        FileName {
            basename: basename,
            extension: extension,
        }
    }
    
    pub fn extension_str(&self) -> &str {
        match &self.extension {
            Some(ext) => ext.as_str(),
            None => "",
        }
    }
}

impl<T> From<T> for FileName where T: Into<String> {
    fn from(name: T) -> Self {
        let name: String = name.into();
        let split = name.rsplit_once(".");

        match split {
            Some((pre, suf)) => FileName::new(pre.to_string(), Some(suf.to_string())),
            None => FileName::new(name, None)
        }
    }
}

impl ToString for FileName {
    fn to_string(&self) -> String {
        match &self.extension {
            Some(ext) => format!("{}.{}", self.basename, ext),
            None => self.basename.clone()
        }
    }
}