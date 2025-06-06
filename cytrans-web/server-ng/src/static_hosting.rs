use std::path::{Component, Path, PathBuf};

use actix_files::NamedFile;
use actix_web::{body::{BoxBody, MessageBody}, http::{header::{AcceptEncoding, ContentEncoding, Encoding, Header, VARY}, StatusCode}, web::Html, HttpRequest, HttpResponse, HttpResponseBuilder, Responder};



fn sanitize_path(input_path: &str) -> Option<PathBuf> {
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

pub async fn serve_static(req: HttpRequest, path_prefix: &Path, strip_prefix: &str) -> HttpResponse<BoxBody> {
    let path = percent_encoding::percent_decode_str(req.path()).decode_utf8();
    let path = match path {
        Ok(x)=>x,
        Err(_) => {
            return HttpResponse::BadRequest().body("Invalid UTF-8 in path");
        }
    };
    dbg!(path.as_ref());
    let path = path.strip_prefix(strip_prefix).expect("request path did not contain the prefix");
    let Some(path) = sanitize_path(path) else {
        return Html::new("<!DOCTYPE html><body><h1>400 Nice Try</h1><p>Surely you didn't think path traversal was going to be <i>that</i> easy.</p></body></html>")
            .customize()
            .with_status(StatusCode::BAD_REQUEST)
            .respond_to(&req)
            .map_body(|_,b|b.boxed());
    };
    let mut path = path_prefix.join(path);
    if path.is_dir() {
        path.push("index.html");
    }
    let path = dbg!(path);
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
            match std::fs::File::open(&path2) {
                Ok(f) => {
                    println!("found brotli encoded file");
                    let f = NamedFile::from_file(f, path).expect("NamedFile::from_file() failed");
                    file = f.set_content_encoding(ContentEncoding::Brotli);
                    must_decompress = match negotiate_compression(&req, Encoding::brotli()) {
                        Ok(x) => x,
                        Err(mut resp) => {
                            resp .append_header((VARY, "Accept-Encoding"));
                            return resp
                                .respond_to(&req)
                                .map_body(|_,b|b.boxed());
                        },
                    }
                }, Err(e) => {
                    println!("couldn't open brotli file {}: {e}", path2.display());
                    return show_404();
                }
            }
        },
        Err(e) => {
            println!("error opening file {}: {e}", path.display());
            return show_404();
        },
    }
    if !must_decompress {
        return file.respond_to(&req);
    } else {
        todo!()
    }
}

pub fn show_404() -> HttpResponse {
    HttpResponse::NotFound().into()
}

fn negotiate_compression(req: &HttpRequest, fs_encoding: Encoding) -> Result<bool, HttpResponseBuilder> {
    if let Ok(accept) = AcceptEncoding::parse(req) {
        match accept.negotiate([&fs_encoding, &Encoding::identity()].into_iter()) {
            Some(Encoding::Known(ContentEncoding::Identity)) => Ok(true),
            Some(e) if e == fs_encoding => Ok(false),
            None => Err(HttpResponse::NotAcceptable()),
            // XXX should this panic?
            _ => Err(HttpResponse::InternalServerError()),
        }
    } else {
        Err(HttpResponse::BadRequest())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn check_sanitize_path(input: &str, expected_output: Option<&Path>) {
        // ugh
        // to think i used to dog on java for having long, verbose, needlessly complex method call chains...
        assert_eq!(sanitize_path(input).as_ref().map(|x|x.as_ref()), expected_output);
    }
    #[test]
    fn test_sanitize_path() {
        check_sanitize_path("../hi", None);
        check_sanitize_path("/hi",Some(Path::new("hi")));
        check_sanitize_path("/hi/.",Some(Path::new("hi")));
        check_sanitize_path("/hi/.secret",None);
        check_sanitize_path("/hi/public.html",Some(Path::new("hi/public.html")));
        check_sanitize_path("/hi/bye",Some(Path::new("hi/bye")));
        check_sanitize_path("/hi/../bye",Some(Path::new("bye")));
        check_sanitize_path("/hi/../../bye",None);
    }
}
