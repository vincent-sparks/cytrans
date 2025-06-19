#![feature(normalize_lexically)]
use std::{ffi::{OsStr, OsString}, net::SocketAddr, path::{Path, PathBuf}, sync::Arc};

use actix_web::{body::{BoxBody, MessageBody}, get, http::{header::{AcceptEncoding, ContentEncoding, Encoding, Header, HeaderName, VARY}, StatusCode}, post, web::{self, Data, Html}, App, HttpRequest, HttpResponse, HttpResponseBuilder, HttpServer, Responder};
use clap::Parser;
use static_hosting::show_404;

mod common;
mod api;
mod noscript;
#[cfg(feature="static_hosting")]
mod static_hosting;

#[get("/hello")]
async fn hello() -> impl Responder {
    Html::new("<!DOCTYPE html><html><head><title>Hello Actix!</title></head><body><p>Hello world!</p></body></html>")
}

#[get("/noscript/browse/{path}")]
async fn browse(path: web::Path<String>, data: Data<Args>) -> impl Responder {
    Html::new(format!("<!DOCTYPE html><html><head><title>Hello Actix!</title></head><body><p>{path:?}</p></body></html>"))
}

#[derive(clap::Parser, Debug)]
#[command(version)]
struct ArgsParsed {
    /// Address to listen on.
    /// May be specified more than once to listen on multiple addresses.  May be a combination of IPv4
    /// and IPv6.  Default is all interfaces, port 8080.
    /// For testing it is recommended to restrict to a loopback address, i.e. [::1]:8080 or 127.0.0.1:8080 
    /// as appropriate.
    #[arg(long="listen",help="Address to listen on",long_help,default_value="[::]:8080")]
    address: Vec<SocketAddr>,
    /// Directory containing input video files to let users pick from.  If unspecified the file
    /// browser will not appear in the webui and cytrans-web will only accept URLs to transcode.
    /// cytrans_web_server will never attempt to write to this directory.
    #[arg(long,long_help)]
    input_dir: Option<PathBuf>,
    /// Directory where transcoded video files should be placed.  This should be exposed to the
    /// Internet via an HTTPS server.
    #[arg(long,long_help)]
    output_dir: PathBuf,
    /// URL prefix that hosts the files placed in the output directory.  cytrans_web_server expects
    /// that if --url-prefix is https://example.com/media/, and it creates the file
    /// $OUTPUT_DIRECTORY/foo/bar.mp4, the file will immediately become available to the public
    /// Internet at https://example.com/media/foo/bar.mp4.  If the URL prefix provided does not end
    /// in a slash, one will be automatically appended.
    /// 
    /// cytrans_web_server does not perform input validation on this value; however, Cytube will not
    /// accept the URLs cytrans generates unless it meets all of the following criteria:  
    ///
    /// * Must be an absolute URL  
    ///
    /// * Must start with https:// (Cytube rejects insecure HTTP URLs)  
    ///
    /// * Certificate must be valid (i.e. not self-signed)  
    ///
    /// * URL must be globally reachable, or at minimum, reachable by the Cytube server and every
    ///   client who intends to watch the video.
    #[arg(long,long_help)]
    url_prefix: String,
    /// Directory containing cytrans-web static files (CSS, WASM, static HTML, etc), if you want
    /// cytrans-web-server to host these as well instead of making e.g. nginx do it.
    #[arg(long,long_help)]
    static_dir: Option<PathBuf>,
}

struct Args {
    input_dir: Option<PathBuf>,
    output_dir: PathBuf,
    url_prefix: String,
    static_dir: Option<PathBuf>,
}
async fn host_static(req: HttpRequest, args: Data<Args>) -> HttpResponse<BoxBody> {
    let Some(ref static_path) = args.static_dir else {
        return show_404();
    };
    static_hosting::serve_static(req, static_path, "").await
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();
    let ArgsParsed {address, input_dir, output_dir, static_dir, url_prefix} = ArgsParsed::parse();
    //let output_dir = sneak::Dir::open(output_dir)?;
    //let input_dir = match input_dir {
    //    Some(x) => Some(sneak::Dir::open(x)?),
    //    None => None,
    //};
    let args = web::Data::new(Args {input_dir, output_dir, static_dir, url_prefix});

    HttpServer::new(move || {
        App::new()
            .app_data(args.clone())
            .service(hello)
            .default_service(web::to(host_static))
    })
    .bind(&*address)?
    .run()
    .await
}
