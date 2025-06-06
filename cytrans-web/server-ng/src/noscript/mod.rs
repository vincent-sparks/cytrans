//! Helpers for server side HTML rendering, for when the client has JavaScript disabled.

use std::ffi::OsString;

use actix_web::web;
use percent_encoding::{AsciiSet, CONTROLS};

const FRAGMENT: &AsciiSet = &CONTROLS.add(b' ').add(b'"').add(b'<').add(b'>').add(b'`');

struct Callback<T>(T);
impl<T> std::fmt::Display for Callback<T> where T: for<'a> Fn(&mut std::fmt::Formatter<'a>) -> std::fmt::Result {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (self.0)(f)
    }
}

// TODO test me
pub fn render_noscript_browse(prefix: OsString, items: Vec<(OsString, bool)>) -> web::Html {
    let prefix_encoded = percent_encoding::percent_encode(prefix.as_encoded_bytes(), FRAGMENT).collect::<String>();
    web::Html::new(format!(
        include_str!("noscript_browse_template.html"),
        entries = Callback(|fmt: &mut std::fmt::Formatter<'_>| {
            for &(ref name, is_dir) in items.iter() {
                fmt.write_fmt(format_args!(include_str!("noscript_browse_entry_template.html"),
                    class=if is_dir {"browse_dir"} else {"browse_movie"},
                    text=name.display(),
                    href_leader=if is_dir {"browse"} else {"ffprobe"},
                    href_prefix=prefix_encoded,
                    href=percent_encoding::percent_encode(name.as_encoded_bytes(), FRAGMENT)
                ))?;
            }
            Ok(())
        })
    ))
}
