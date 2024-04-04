use cytrans::codecs::Capabilities;
use serde::{ser::SerializeMap, de::DeserializeSeed};
use axum::{extract::{FromRequest, FromRequestParts}, response::{Response, IntoResponse}};
use async_trait::async_trait;
use http::request::Request;
use http::{StatusCode, HeaderValue};
use http::header::{CONTENT_TYPE, ACCEPT};
use std::convert::Infallible;

pub struct JsonifiableCapabilities<'a>(pub &'a Capabilities);
struct JsonifiableCapSet<'a, T>(&'a Vec<(T, Vec<String>)>);

impl<'a, T> serde::Serialize for JsonifiableCapSet<'a, T> where T: AsRef<str> {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut s = ser.serialize_map(Some(self.0.len()))?;
        for (k,v) in self.0.iter() {
            s.serialize_entry(k.as_ref(), v)?;
        }
        s.end()
    }
}

impl<'a> serde::Serialize for JsonifiableCapabilities<'a> {
    fn serialize<S: serde::Serializer>(&self, ser: S) -> Result<S::Ok, S::Error> {
        let mut s = ser.serialize_map(Some(2))?;
        s.serialize_entry("video", &JsonifiableCapSet(&self.0.0))?;
        s.serialize_entry("audio", &JsonifiableCapSet(&self.0.1))?;
        s.end()
    }
}

pub struct JsonOrPostcard<T>(pub T);
pub struct JsonOrPostcardSeed<'a,T>(pub T::Value) where T: DeserializeSeed<'a>;
pub struct JsonOrPostcardResponse<T>(pub Which, pub T);

pub enum JsonOrPostcardRejection<ES> {
    InvalidContentType,
    StateFailure(ES),
    ReadFailure(axum::extract::rejection::BytesRejection),
    JsonError(serde_json::Error),
    PostcardError(postcard::Error),
}

impl<ES> IntoResponse for JsonOrPostcardRejection<ES> where ES: IntoResponse {
    fn into_response(self) -> axum::response::Response {
        use JsonOrPostcardRejection::*;
        match self {
            InvalidContentType => (StatusCode::BAD_REQUEST, "Content type must be application/json or application/x-postcard").into_response(),
            StateFailure(e) => e.into_response(),
            ReadFailure(e) => e.into_response(),
            JsonError(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
            PostcardError(e) => (StatusCode::BAD_REQUEST, e.to_string()).into_response(),
        }
    }
}

pub enum Which {
    Json,
    Postcard
}

#[async_trait]
impl<S,B,T> FromRequest<S,B> for JsonOrPostcard<T> where 
        T: for<'a> serde::Deserialize<'a>,
        B: axum::body::HttpBody + Send + Sync + 'static,
        B::Error: Send + Sync + std::error::Error,
        B::Data: Send,
{
    type Rejection = JsonOrPostcardRejection<Infallible>;
    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let which = match_content_type(req.headers().get(CONTENT_TYPE)).ok_or(JsonOrPostcardRejection::InvalidContentType)?;
        let body = axum::body::Bytes::from_request(req,&()).await.map_err(JsonOrPostcardRejection::ReadFailure)?;

        match which {
            Which::Json => {
                let deserializer = &mut serde_json::Deserializer::from_slice(&body);
                T::deserialize(deserializer).map(Self).map_err(JsonOrPostcardRejection::JsonError)
            },
            Which::Postcard => {
                T::deserialize(&mut postcard::Deserializer::from_bytes(&body))
                    .map(Self)
                    .map_err(JsonOrPostcardRejection::PostcardError)
            },
        }
    }
}

#[async_trait]
impl<S,B,D,T> FromRequest<S,B> for JsonOrPostcardSeed<'_, D> where 
        D: Send + for<'a> serde::de::DeserializeSeed<'a, Value=T> + FromRequestParts<S>,
//        for<'a>  <D as DeserializeSeed<'a>>::Value: 'static,
        B: axum::body::HttpBody + Send + Sync + 'static,
        B::Error: Send + Sync + std::error::Error,
        B::Data: Send,
        S: Send + Sync,
{
    type Rejection = JsonOrPostcardRejection<D::Rejection>;
    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
        let which = match_content_type(req.headers().get(CONTENT_TYPE)).ok_or(JsonOrPostcardRejection::InvalidContentType)?;

        let (mut parts, body) = req.into_parts();
        let seed = D::from_request_parts(&mut parts, state).await.map_err(JsonOrPostcardRejection::StateFailure)?;

        let req = Request::from_parts(parts, body);

        Self::parse_request(seed, req).await.map(Self)
    }
}

impl<'a, D,T> JsonOrPostcardSeed<'a, D> where
    D: Send + for<'b> serde::de::DeserializeSeed<'b, Value=T>,
{
    pub async fn parse_request<B, E>(seed: D, req: Request<B>) -> Result<T, JsonOrPostcardRejection<E>> where
        B: axum::body::HttpBody + Send + Sync + 'static,
        B::Error: Send + Sync + std::error::Error,
        B::Data: Send
    {
        let which = match_content_type(req.headers().get(CONTENT_TYPE)).ok_or(JsonOrPostcardRejection::InvalidContentType)?;
        let body = axum::body::Bytes::from_request(req,&()).await.map_err(JsonOrPostcardRejection::ReadFailure)?;
        match which {
            Which::Json => {
                let deserializer = &mut serde_json::Deserializer::from_slice(&body);
                seed.deserialize(deserializer).map_err(JsonOrPostcardRejection::JsonError)
            },
            Which::Postcard => {
                seed.deserialize(&mut postcard::Deserializer::from_bytes(&body))
                    .map_err(JsonOrPostcardRejection::PostcardError)
            },
        }
    }
}

impl<T> IntoResponse for JsonOrPostcardResponse<T> where
    T: serde::Serialize
{
    fn into_response(self) -> Response {
        let body = match self.0 {
            Which::Json => {
                match serde_json::to_vec(&self.1) {
                    Ok(x) => x,
                    Err(e) => return JsonOrPostcardRejection::<Infallible>::JsonError(e).into_response(),
                }
            },
            Which::Postcard => {
                match postcard::to_allocvec(&self.1) {
                    Ok(x) => x,
                    Err(e) => return JsonOrPostcardRejection::<Infallible>::PostcardError(e).into_response(),
                }
            },
        };
        return Response::builder()
            .status(200)
            .header(CONTENT_TYPE, match self.0 {
                Which::Json => "application/json",
                Which::Postcard => "application/x-postcard",
            })
            .body(axum::body::boxed::<axum::body::Body>(body.into())).unwrap();
    }
}

fn match_content_type(ct: Option<&HeaderValue>) -> Option<Which> {
    let s = ct?.to_str().ok()?.strip_prefix("application/")?;
    if s == "x-postcard" {
        return Some(Which::Postcard);
    } else if s.contains("json") {
        return Some(Which::Json);
    }
    else {
        return None;
    }
}

#[async_trait]
impl<S> FromRequestParts<S> for Which {
    type Rejection = Infallible;
    // if the client doesn't specify either postcard or json, default to json, so that browsers
    // making requests from their address bar get something they can use.
    async fn from_request_parts(parts: &mut http::request::Parts, _state: &S) -> Result<Self, Infallible> {
        Ok(match_content_type(parts.headers.get(ACCEPT)).unwrap_or(Which::Json))
    }
}
