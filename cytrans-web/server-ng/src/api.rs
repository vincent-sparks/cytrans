use crate::common::{self, BrowseError};

use actix_web::{body::BoxBody, get, web::{self, Data}, HttpResponse, Responder, ResponseError};

#[get("/api/browse?path={path}")]
fn browse(path: web::Path<String>, data: Data<crate::ArgsParsed>) -> Result<impl Responder, BrowseError> {
    Ok("hi")
}
