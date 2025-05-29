use std::net::{Ipv6Addr, SocketAddrV6};

use actix_web::{get, post, web::{self, Html}, App, HttpServer, Responder};

#[get("/hello")]
async fn hello() -> impl Responder {
    Html::new("<!DOCTYPE html><html><head><title>Hello Actix!</title></head><body><p>Hello world!</p></body></html>")
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let ip_addr = SocketAddrV6::new(Ipv6Addr::LOCALHOST, 8080, 0, 0);
    HttpServer::new(|| {
        App::new()
            .service(hello)
    })
    .bind(ip_addr)?
    .run()
    .await
}
