use std::{net::{Ipv6Addr, SocketAddr, SocketAddrV6}, path::PathBuf};

use actix_web::{get, post, web::{self, Html}, App, HttpServer, Responder};
use clap::Parser;

#[get("/hello")]
async fn hello() -> impl Responder {
    Html::new("<!DOCTYPE html><html><head><title>Hello Actix!</title></head><body><p>Hello world!</p></body></html>")
}

#[derive(clap::Parser, Debug)]
#[command(version)]
struct Args {
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
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let args = Args::parse();
    HttpServer::new(|| {
        App::new()
            .service(hello)
    })
    .bind(&*args.address)?
    .run()
    .await
}
