use axum::{Router, routing::{get, post}, extract::{Query, RawBody}};
use http::header::CONTENT_TYPE;
use std::net::SocketAddr;
mod ser;
mod tr_deserialize;
mod state;
mod macros;
use std::sync::Arc;
use axum::{response::{Response, IntoResponse}, body::{Body, Bytes}};
use axum::extract::{Json, ws::WebSocketUpgrade, FromRequestParts};
use http::request::Request;

use cytrans::codecs::{Capabilities, get_capabilities};
use cytrans::ffprobe::FFprobeResult;

use postcard::to_allocvec;

use crate::ser::{JsonOrPostcardSeed, JsonOrPostcardResponse};
use crate::tr_deserialize::TranscodeArgsDeserializer;
use crate::state::State as MyState;
use crate::state::{PathKind, BadPath, FFprobeError};
use axum::extract::State;

use log::{info};


#[derive(serde::Deserialize)]
struct BrowseArgs{path: String}

macro_rules! try_or_continue {
    ($e: expr) => {
        match $e {
            Ok(x) => x,
            Err(e) => {
                eprintln!("ignoring error {}", e);
                continue
            },
        }
    }
}

macro_rules! or_continue {
    ($e: expr) => {
        match $e {
            Some(x) => x,
            None => continue,
        }
    }
}

rejection_enum!(FFprobeRejection, {FFprobeError, BadPath});

async fn ffprobe(Query(BrowseArgs{path}): Query<BrowseArgs>, State(s): State<Arc<MyState>>, which: ser::Which) -> Result<JsonOrPostcardResponse<Arc<FFprobeResult>>, FFprobeRejection> {
    let path = s.sanitize_path(&path, PathKind::Input)?;
    Ok(JsonOrPostcardResponse(which, s.ffprobe(&path)?))
}

async fn browse(Query(BrowseArgs{path}): Query<BrowseArgs>, State(s): State<Arc<MyState>>) -> Result<Response<Body>, BadPath>  {
    let path = s.sanitize_path(&path, PathKind::Input)?;
    let r = Response::builder().header(CONTENT_TYPE, "text/plain; charset=utf-8");
    Ok({
        match path.read_dir() {
            Ok(it) => {
                let mut v = Vec::new();
                for item in it {
                    // if we encounter an error, just skip it.
                    // if we get errors accessing these files now (likely due to permisisons) we
                    // DEFINITELY will when the user tries to access them.
                    let item = try_or_continue!(item);
                    let is_dir = try_or_continue!(item.file_type()).is_dir();
                    
                    let filename = item.file_name();
                    // don't show files whose names aren't valid Unicode.  I don't want to deal with
                    // that right now.  Or possibly ever.
                    let filename = or_continue!(filename.to_str());
                    // Also files whose names contain newlines.
                    if filename.contains('\n') {
                        continue;
                    }
                    let mut name = filename.to_owned();
                    if is_dir {
                        name.push('/');
                    }
                    v.push(name);
                }
                v.sort();
                r.status(200).body(v.join("\n").into())
            },
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                r.status(404).body("Not Found".into())
            }
            Err(e) => {
                r.status(400).body(e.to_string().into())
            }
        }
    }.unwrap())
}

async fn queue(State(state): State<Arc<MyState>>, req: Request<Body>) -> Response {
    let (mut parts, _body) = req.into_parts();
    if let Ok(wsu) = WebSocketUpgrade::from_request_parts(&mut parts, &()).await {
        wsu.on_upgrade(|ws| async {
            
        })
    } else {
        Json(state.queue().await).into_response()
    }
}

macro_rules! ir_try {
    ($e:expr) => {
        match $e {
            Ok(x)=>x,
            Err(e)=>return e.into_response(),
        }
    }
}

async fn launch_ffmpeg<'a>(Query(BrowseArgs{path}): Query<BrowseArgs>, State(state): State<Arc<MyState>>, which: ser::Which, request: Request<Body>) -> Response {
    let path = ir_try!(state.sanitize_path(&path, PathKind::Input));
    let ffprobe = ir_try!(state.ffprobe(&path));
    let args = ir_try!(JsonOrPostcardSeed::parse_request::<_, std::convert::Infallible>(TranscodeArgsDeserializer{tracks: &ffprobe.tracks, duration: ffprobe.duration}, request).await);
    let position = ir_try!(state.queue_job(&path, args, "test".to_string()).await);
    JsonOrPostcardResponse(which, position).into_response()
}

#[tokio::main]
async fn main() {
    env_logger::init();
    let app= Router::new()
        .route("/", get(|| async{"Hello Axum!"}))
        .route("/capabilities", get(|| async{axum::Json(ser::JsonifiableCapabilities(get_capabilities()))}))
        .route("/capa", get(||async{to_allocvec(get_capabilities()).unwrap()}))
        .route("/files", get(browse))
        .route("/ffprobe", get(ffprobe))
        .route("/go", post(launch_ffmpeg))
        .route("/queue", get(queue));
    let app=app
        .with_state(MyState::new("/home/seanw", "/home/seanw/Videos/cytrans_out", "https://red.baka.haus/"));
    info!("starting up");
    axum::Server::bind(&SocketAddr::from(([127,0,0,1],27107))).serve(app.into_make_service()).await.unwrap();
}
