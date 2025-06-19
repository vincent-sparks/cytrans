use std::path::{Component, Path, PathBuf};

enum Entry {
    Dir(String),
    File(String),
}

enum SanitizePathError {
    AttemptedRootTraversal,
    IllegalCharacter(char),
}

enum BrowseError {
    NotFound,
    IoError(std::io::Error),
    NaughtyPath,
}

impl From<std::io::Error> for BrowseError {
    fn from(e:std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::NotFound {
            Self::NotFound
        } else {
            Self::IoError(e)
        }
    }
}

const MOVIE_EXTS: [&str;3]=[".mp4",".mkv",".webm"];

pub fn sanitize_path(input_path: &str) -> Option<PathBuf> {
    let mut it = std::path::Path::new(input_path).components();
    let mut res = PathBuf::new();
    let Some(first_component) = it.next() else {
        return Some(res);
    };
    match first_component {
        Component::RootDir | Component::CurDir => {},
        Component::Normal(os_str) => {res.push(os_str)},
        _ => return None,
    }

    for item in it {
        match item {
            Component::CurDir => {},
            Component::ParentDir => {
                if !res.pop() {
                    // we could no-op in this case and still be compliant,
                    // but for personal reasons i would prefer to show a cheeky message to anyone
                    // who tries path traversal, so we return None here.
                    return None;
                };
            },
            Component::Normal(path) => {
                if path.as_encoded_bytes().starts_with(b".") {
                    // dotfiles are not to be served!
                    return None;
                }
                res.push(path)
            },
            _ => return None,
        }
    }

    Some(res)
}

pub fn browse(input_path: &Path, browse_path: &str) -> Result<Vec<Entry>, BrowseError> {
    let p = sanitize_path(browse_path).ok_or(BrowseError::NaughtyPath)?;
    let p = input_path.join(p);
    let mut v = Vec::new();
    for entry in std::fs::read_dir(&p)? {
        let entry = entry.map_err(BrowseError::IoError)?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if name.starts_with('.') {
            continue;
        }
        let meta = std::fs::metadata(&p)?;
        if meta.is_dir(){
            v.push(Entry::Dir(name.into()));
        } else if meta.is_file() && MOVIE_EXTS.iter().any(|ext| name.ends_with(ext)) {
            v.push(Entry::File(name.into()));
        }

    }
    Ok(v)
}
