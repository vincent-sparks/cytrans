use crate::common;

use actix_web::{get, web};

#[get("/api/browse?path={path}")]
fn browse(path: web::Path<String>) {
}
