use std::{borrow::Cow, path::{Component, Path, PathBuf}};

use actix_web::{body::BoxBody, http::{header::{ContentType, TryIntoHeaderValue as _}, StatusCode}, ResponseError};

enum Entry {
    Dir(String),
    File(String),
}

#[derive(Debug, thiserror::Error)]
#[cfg_attr(test, derive(PartialEq))]
pub enum SanitizePathError {
    #[error("Attempted path traversal")]
    AttemptedRootTraversal,
    #[error("Access to hidden files is not allowed")]
    AttemptedHiddenFileAccess,
    #[error("Invaid character '{0}' in path")]
    IllegalCharacter(char),
    #[error("Invalid UTF8 in path")]
    InvalidUTF8,
}

#[derive(Debug, thiserror::Error)]
pub enum BrowseError {
    #[error("{0}")]
    IoError(#[from] std::io::Error),
    #[error("{0}")]
    NaughtyPath(#[from] SanitizePathError),
}

impl ResponseError for SanitizePathError {
    fn status_code(&self) -> StatusCode {
        match self {
            SanitizePathError::AttemptedRootTraversal => StatusCode::BAD_REQUEST,
            SanitizePathError::AttemptedHiddenFileAccess => StatusCode::FORBIDDEN,
            SanitizePathError::IllegalCharacter(_) => StatusCode::BAD_REQUEST,
            SanitizePathError::InvalidUTF8 => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> actix_web::HttpResponse<BoxBody> {
        let mut res = actix_web::HttpResponse::new(self.status_code());

        if matches!(self, Self::AttemptedRootTraversal) {
            return actix_web::HttpResponse::BadRequest()
                .reason("Nice Try") // yes this technically violates RFC.  so what?  it's never going to be shown to anyone who isn't looking for an exploit.  plus i finally have a good place to hide an easter egg in one of my programs :)
                .append_header(ContentType::html())
                .body(BoxBody::new("<!DOCTYPE html><body><h1>400 Nice Try</h1><p>Surely you didn't think path traversal was going to be <i>that</i> easy.</p></body></html>"));
        }

        let mime = actix_web::mime::TEXT_PLAIN_UTF_8.try_into_value().unwrap();
        res.headers_mut().insert(actix_web::http::header::CONTENT_TYPE, mime);

        res.set_body(BoxBody::new(self.to_string()))
    }
}

impl ResponseError for BrowseError {
    fn status_code(&self) -> StatusCode {
        match self {
            BrowseError::IoError(error) => error.status_code(),
            BrowseError::NaughtyPath(error) => error.status_code(),
        }
    }
    fn error_response(&self) -> actix_web::HttpResponse {
        match self {
            BrowseError::IoError(error) => error.error_response(),
            BrowseError::NaughtyPath(error) => error.error_response(),
        }
    }
}

const MOVIE_EXTS: [&str;3]=[".mp4",".mkv",".webm"];

pub fn decode_path(percent_encoded: &str) -> Result<Cow<'_, str>, SanitizePathError> {
    let decoded = percent_encoding::percent_decode_str(percent_encoded).decode_utf8().map_err(|_| SanitizePathError::InvalidUTF8)?;
    // only bother to recount the slashes if percent decode actually expanded anything.
    if let Cow::Owned(ref dec) = decoded {
        let count = percent_encoded.as_bytes().iter().filter(|x| **x==b'/').count();
        let count2 = dec.as_bytes().iter().filter(|x| **x==b'/').count();
        if count != count2 {
            // forbid percent-encoding slashes
            return Err(SanitizePathError::IllegalCharacter('/'));
        }
    }
    Ok(decoded)
}

// TODO UPDATE ME
// THIS FUNCTION IS MISSING SEVERAL IMPORTANT CHECKS THAT STILL NEED TO BE PORTED FROM actix_files
pub fn sanitize_path(input_path: &str) -> Result<PathBuf, SanitizePathError> {
    let mut it = std::path::Path::new(input_path).components();
    let mut res = PathBuf::new();
    let Some(first_component) = it.next() else {
        return Ok(res);
    };
    match first_component {
        Component::RootDir | Component::CurDir => {},
        Component::Normal(os_str) => {res.push(os_str)},
        _ => return Err(SanitizePathError::AttemptedRootTraversal),
    }

    for item in it {
        match item {
            Component::CurDir => {},
            Component::ParentDir => {
                if !res.pop() {
                    // we could no-op in this case and still be compliant,
                    // but for personal reasons i would prefer to show a cheeky message to anyone
                    // who tries path traversal, so we return None here.
                    return Err(SanitizePathError::AttemptedRootTraversal);
                };
            },
            Component::Normal(path) => {
                if path.as_encoded_bytes().starts_with(b".") {
                    // dotfiles are not to be served!
                    return Err(SanitizePathError::AttemptedHiddenFileAccess);
                }
                res.push(path)
            },
            _ => return Err(SanitizePathError::AttemptedRootTraversal),
        }
    }

    Ok(res)
}

pub fn browse(input_path: &Path, browse_path: &str) -> Result<Vec<Entry>, BrowseError> {
    let p = sanitize_path(browse_path)?;
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
