use std::path::Path;

use actix_files::NamedFile;
use actix_web::{body::BoxBody, http::{header::{self, AcceptEncoding, ContentEncoding, Encoding, Header, TryIntoHeaderValue as _, RANGE}, StatusCode}, HttpRequest, HttpResponse, Responder, ResponseError};

use crate::common::{decode_path, sanitize_path, SanitizePathError};

#[derive(thiserror::Error,Debug)]
pub enum ServeStaticError {
    #[error("{0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    NaughtyPath(#[from] SanitizePathError),
    #[error("No acceptable Content-Encoding could be determined.  Something is deeply wrong with your HTTP client.")]
    NotAcceptable,
    #[error("Invalid Accept-Encoding header.  Something is deeply wrong with your HTTP client.")]
    InvalidAcceptHeader,
}

impl ResponseError for ServeStaticError {
    fn status_code(&self) -> StatusCode {
        match self {
            Self::Io(e) => e.status_code(),
            Self::NaughtyPath(e) => e.status_code(),
            Self::NotAcceptable => StatusCode::NOT_ACCEPTABLE,
            Self::InvalidAcceptHeader => StatusCode::BAD_REQUEST,
        }
    }

    fn error_response(&self) -> HttpResponse<BoxBody> {
        match self {
            Self::NaughtyPath(e) => e.error_response(),
            Self::Io(e) => e.error_response(),
            _ => {
                let mut res = HttpResponse::new(self.status_code());

                let mime = actix_web::mime::TEXT_PLAIN_UTF_8.try_into_value().unwrap();
                res.headers_mut().insert(header::CONTENT_TYPE, mime);
                res.headers_mut().insert(header::VARY, header::ACCEPT_ENCODING.into());

                res.set_body(BoxBody::new(self.to_string()))
            }
        }
    }
}

/// "But wait!" I hear you say. "Doesn't actix_files have this built in?"
/// 
/// Yes, dear reader! Yes it does!  It's right there in [`actix_files::Files`]!  Unfortunately,
/// that code does not have support for files that are compressed on disk, and does not have
/// support for adding callbacks to change how files are found! The only way to implement that using
/// the Files handler would be to write a 404 handler that reads the URL path and searches for
/// a Brotli file under the same path on disk. Unfortunately, since the 404 handler doesn't get
/// access to the disk path that was searched, we would have to reimplement all the path
/// translation from scratch... and at that point, why bother using `actix_files::Files` at all?
pub async fn serve_static(req: HttpRequest, path_prefix: &Path, strip_prefix: &str) -> Result<HttpResponse<BoxBody>, ServeStaticError> {
    dbg!(req.path());
    let path = decode_path(req.path())?;
    let path = path.strip_prefix(strip_prefix).expect("request path did not contain the prefix");
    let path = sanitize_path(path)?;
    let mut path = path_prefix.join(path);
    if path.is_dir() {
        path.push("index.html");
    }
    let file;
    let must_decompress;
    match actix_files::NamedFile::open_async(&path).await {
        Ok(f) => {
            file = f;
            must_decompress = false;
        },
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            let mut path2 = path.clone();
            path2.as_mut_os_string().push(".br");
            let f = std::fs::File::open(&path2)?;

            file = NamedFile::from_file(f, path)
                .expect("NamedFile::from_file() failed")
                .set_content_encoding(ContentEncoding::Brotli);
            must_decompress = negotiate_compression(&req, Encoding::brotli())? | req.headers().contains_key(RANGE);
        },
        Err(e) => {
            return Err(e.into());
        },
    }
    if !must_decompress {
        Ok(file.respond_to(&req))
    } else {
        todo!()
    }
}

pub fn show_404() -> HttpResponse {
    HttpResponse::NotFound().into()
}

fn negotiate_compression(req: &HttpRequest, fs_encoding: Encoding) -> Result<bool, ServeStaticError> {
    if let Ok(accept) = AcceptEncoding::parse(req) {
        match accept.negotiate([&fs_encoding, &Encoding::identity()].into_iter()) {
            Some(Encoding::Known(ContentEncoding::Identity)) => Ok(true),
            Some(e) if e == fs_encoding => Ok(false),
            None => Err(ServeStaticError::NotAcceptable),
            _ => unreachable!(),
        }
    } else {
        Err(ServeStaticError::InvalidAcceptHeader)
    }
}

#[cfg(test)]
mod test {
    use std::path::Path;

    use super::*;

    fn check_sanitize_path(input: &str, expected_output: Result<&Path, SanitizePathError>) {
        // ugh
        // to think i used to dog on java for having long, verbose, needlessly complex method call chains...
        assert_eq!(sanitize_path(input).as_ref().map(|x|x.as_ref()), expected_output.as_ref().map(|x|*x));
    }
    #[test]
    fn test_sanitize_path() {
        check_sanitize_path("../hi", Err(SanitizePathError::AttemptedRootTraversal));
        check_sanitize_path("/hi",Ok(Path::new("hi")));
        check_sanitize_path("/hi/.",Ok(Path::new("hi")));
        check_sanitize_path("/hi/.secret",Err(SanitizePathError::AttemptedHiddenFileAccess));
        check_sanitize_path("/hi/public.html",Ok(Path::new("hi/public.html")));
        check_sanitize_path("/hi/bye",Ok(Path::new("hi/bye")));
        check_sanitize_path("/hi/../bye",Ok(Path::new("bye")));
        check_sanitize_path("/hi/../../bye",Err(SanitizePathError::AttemptedRootTraversal));
    }
}
