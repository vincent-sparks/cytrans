use crate::common::{self, BrowseError};

use actix_web::{body::BoxBody, get, web::{self, Data}, HttpResponse, Responder, ResponseError};

async fn browse(path: web::Path<String>, data: Data<crate::ArgsParsed>) -> Result<BrowseResult, BrowseError> {
    
}
