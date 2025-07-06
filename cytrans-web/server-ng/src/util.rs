use actix_web::{body::{BoxBody, EitherBody, MessageBody}, http::{header::{self, Accept, ContentType, Header}, StatusCode}, mime::Mime, HttpResponse, HttpResponseBuilder, Responder};



// This trait has a 'static bound because rustc is too stupid to understand that you can have an
// &'static reference to a function pointer even if that function has PARAMETER TYPES that don't
// outlive 'static.
pub trait AcceptAwareResponse: 'static {
    type Body: MessageBody = BoxBody;
    const FORMATS: &[(Mime, fn(Self) -> Self::Body)];
    fn builder(&self) -> HttpResponseBuilder {
        HttpResponse::Ok()
    }
}

pub struct AARWrapper<T>(pub T);

impl<T> Responder for AARWrapper<T> where T: AcceptAwareResponse {
    type Body=EitherBody<T::Body,String>;

    fn respond_to(self, req: &actix_web::HttpRequest) -> actix_web::HttpResponse<Self::Body> {
        let Ok(accept_header) = Accept::parse(req) else {
            return HttpResponse::BadRequest()
                .insert_header((header::VARY, "Accept"))
                .insert_header(ContentType::plaintext())
                .message_body("Your Accept: header failed to parse".to_string())
                .expect("error generating error response")
                .map_into_right_body();
        };
        let preferences = accept_header.ranked();
        let mut builder = self.0.builder();
        builder.insert_header((header::VARY, "Accept"));
        if let Some((mime, body_fn)) = T::FORMATS
            .iter()
            .filter_map(|(mime, func)| mime_score(mime, &preferences).map(|score| (score, (mime, func))))
            .min_by_key(|(score, _)| *score) 
            .map(|(_, val)| val)
        {
            builder.insert_header((header::CONTENT_TYPE, mime.clone()));
            builder.message_body((body_fn)(self.0)).expect("error from response builder").map_into_left_body()
        } else {
            HttpResponse::NotAcceptable()
                .insert_header((header::VARY, "Accept"))
                .insert_header(ContentType::plaintext())
                .message_body(format!("No document matching your Accept: header could be found. Available MIME types: {}", 
                        T::FORMATS.iter()
                        .map(|x| x.0.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")))
                .expect("error generating error response")
                .map_into_right_body()
        }
    }
}

pub(crate) fn mime_score(mime: &Mime, preferences: &[Mime]) -> Option<usize> {
    let mut best = None;
    let mut found_matching_subtype_yet = false;

    for (i, preference) in preferences.iter().enumerate() {
        if preference.type_() == mime.type_() {
            if preference.subtype() == mime.subtype() {
                return Some(i);
            }
            if preference.subtype() == "*" {
                best = Some(i);
                found_matching_subtype_yet = true;
            }
        } else if preference.type_() == "*" && !found_matching_subtype_yet {
            best = Some(i);
        }
    }

    best
}

#[cfg(test)]
pub(crate) mod test {
    use super::*;
    use actix_web::{mime, test::TestRequest, web::Bytes, HttpRequest};

    pub(crate) fn generate_test_request(accept: &str) -> HttpRequest {
        TestRequest::get().insert_header((header::ACCEPT, accept)).to_http_request()
    }

    struct TestResponse;
    fn json_response(_:TestResponse) -> &'static str {
        r#"{"hello":"world"}"#
    }
    fn plain_response(_:TestResponse) -> &'static str {
        "Hello World!"
    }
    fn html_response(_:TestResponse) -> &'static str {
        "<!DOCTYPE html><html><body><h1>Hello world!</h1></body></html>"
    }
    impl AcceptAwareResponse for TestResponse {
        type Body = &'static str;

        const FORMATS: &[(Mime, fn(Self) -> Self::Body)] = &[
            (mime::TEXT_PLAIN_UTF_8, plain_response),
            (mime::APPLICATION_JSON, json_response),
            (mime::TEXT_HTML_UTF_8, html_response)
        ];

        fn builder(&self) -> HttpResponseBuilder {
            HttpResponse::Ok()
        }
    }

    fn run_test(req: HttpRequest, expected_content_type: &str, expected_body: &str) {
        let resp = AARWrapper(TestResponse).respond_to(&req);

        assert_eq!(resp.headers().get(header::VARY).expect("no Vary header"), "Accept");

        let response_content_type = resp.headers().get(header::CONTENT_TYPE).expect("missing Content-Type header");
        assert_eq!(response_content_type, expected_content_type);

        let body = resp.into_body().try_into_bytes().expect("should not produce a streaming body");
        let body = std::str::from_utf8(&body).expect("body was not valid utf8");
        assert_eq!(body, expected_body);
    }

    #[test]
    fn test() {
        run_test(generate_test_request("*/*"), "text/plain; charset=utf-8", "Hello World!");
        run_test(generate_test_request("application/json"), "application/json", r#"{"hello":"world"}"#);
        run_test(generate_test_request("text/html,*/*;q=0.8"), "text/html; charset=utf-8", "<!DOCTYPE html><html><body><h1>Hello world!</h1></body></html>");
    }
    #[test]
    fn test_no_accept_header() {
        run_test(TestRequest::get().to_http_request(), "text/plain; charset=utf-8", "No document matching your Accept: header could be found. Available MIME types: text/plain; charset=utf-8, application/json, text/html; charset=utf-8");
    }
}
