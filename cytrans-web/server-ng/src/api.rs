use crate::common::{self, BrowseResult, BrowseError, PathParam};

use actix_web::{body::BoxBody, get, web::{self, Data, Query}, HttpResponse, Responder, ResponseError};

#[get("/api/browse")]
pub async fn browse(Query(PathParam{path}): Query<PathParam>, data: Data<crate::Args>) -> Result<BrowseResult, BrowseError> {
    crate::common::browse(data, &path)
}
