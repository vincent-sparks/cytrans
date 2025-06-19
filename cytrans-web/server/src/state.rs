use crate::ser::*;
use crate::tr_deserialize::TranscodeArgsDeserializer;
use async_trait::async_trait;
use quick_cache::sync::Cache;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, broadcast,  watch, Notify, RwLock};
use tokio::process::Command;
use std::collections::VecDeque;
use std::process::Stdio;
use tokio::io::AsyncBufReadExt;
use std::path::Component;
use log::{debug, info, warn, error};
use anyhow::anyhow;

use cytrans::ffprobe::{ffprobe, FFprobeResult};
use cytrans::options::TranscodeArgs;
use cytrans::transcode::{build_ffmpeg_command, build_demux_commands};
use cytrans::metadata::MetadataManifest;

use cytrans_ws::NetworkTranscodeArgs;

pub enum TranscodeStatus {
    Idle, Working{
        input_file: String,
        slug: String,
        duration: f32,
        progress: watch::Receiver<f32>,
        output: Arc<RwLock<Vec<String>>>,
        output_notify: Arc<Notify>,
    }
}

use lazy_static::lazy_static;
use regex::Regex;

struct TranscodeJob {
    command: Command,
    slug: String,
    duration: f32,
}

pub(crate) struct State {
    media_prefix: PathBuf,
    output_prefix: PathBuf,
    url_prefix: String,
    ffprobe_cache: Arc<Cache<PathBuf, Arc<FFprobeResult>>>,
    queue: RwLock<VecDeque<(PathBuf, NetworkTranscodeArgs)>>,
    queue_notify: Notify,
    status: watch::Receiver<TranscodeStatus>,
}

pub enum PathKind {
    Input, Output
}

impl State {
    pub fn new(media_prefix: impl Into<PathBuf>, output_prefix: impl Into<PathBuf>, url_prefix: impl Into<String>) -> Arc<Self> {
        let (status_sender, status_receiver) = watch::channel(TranscodeStatus::Idle);

        let self_ = Arc::new(State {
            media_prefix: media_prefix.into(),
            output_prefix: output_prefix.into(),
            url_prefix: url_prefix.into(),
            ffprobe_cache: Arc::new(Cache::new(30)), // cache size of 30 chosen arbitrarily
            queue: RwLock::new(VecDeque::new()),
            queue_notify: Notify::new(),
            status: status_receiver,
        });
        tokio::spawn(self_.clone().process_queue(status_sender));

        self_
    }

    // i am paranoid about this function and am worried that there is an exploit in it
    pub fn sanitize_path(&self, input: &str, kind: PathKind) -> Result<PathBuf, BadPath> {
        let p: &Path = input.as_ref();
        if p.components().into_iter().any(|x| x==Component::ParentDir) {
            return Err(BadPath::AttemptedDirectoryTraversal);
        }
        let p = match p.strip_prefix("/") {
            Ok(stripped) => stripped,
            Err(_) => p,
        };
        let ref pfx = match kind {
            PathKind::Input => &self.media_prefix,
            PathKind::Output => &self.output_prefix,
        };
        Ok(pfx.join(p))
    }

    pub fn ffprobe(&self, path: &Path) -> Result<Arc<FFprobeResult>, FFprobeError> {
        let path = self.media_prefix.join(path);
        let cache = self.ffprobe_cache.clone();
        match cache.get(&path) {
            Some(x) => Ok(x),
            None => {
                let res = Arc::new(ffprobe(&path).map_err(FFprobeError::FFprobeFailed)?);
                cache.insert(path, res.clone());
                Ok(res)
            },
        }
    }

    pub async fn queue(&self) -> Vec<String> {
        self.queue.read().await.iter().map(|x|x.1.slug.clone()).collect()
    }

    async fn process_queue(self: Arc<Self>, status_sender: watch::Sender<TranscodeStatus>) {
        loop {
            while let Some((path, job)) = self.queue.write().await.pop_front() {
                let ffprobe = match self.ffprobe(&path) {
                    Ok(x) => x,
                    Err(e) => {
                        // TODO report error to client.
                        continue;
                    }
                };

                let (job, metadata, did_demux) = build_ffmpeg_command(&path, job.slug, &self.sanitize_path(&job.slug, PathKind::Output).unwrap());

                if did_demux {
                }

                let output = Arc::new(RwLock::new(Vec::new()));
                let output_notify = Arc::new(Notify::new());
                let (progress_sender, progress_receiver) = watch::channel(0.0f32);
                let status = TranscodeStatus::Working {
                    output: output.clone(),
                    output_notify: output_notify.clone(),
                    progress: progress_receiver,
                    slug: job.slug.clone(),
                    duration: ffprobe.duration,
                    input_file: "".into(),
                };
                let _ = status_sender.send(status);
                if let Err(e) = self.run_ffmpeg(&path, job).await {
                    error!("ffmpeg failed: {}.  todo notify clients.", e);
                }
            }
            let _ = status_sender.send(TranscodeStatus::Idle);
            self.queue_notify.notified().await;
        }
    }

    async fn demux(&self, slug: &str) {
        let meta = match self.get_meta(slug) {
            Some(x)=>x,
            None=>return,
        };

    }

    fn get_meta(&self, slug: &str) -> Option<&mut MetadataManifest> {
        // TODO
        None
    }
    
    /*
    fn aaa() {
        let outputdir = self.sanitize_path(&job.slug, PathKind::Output)?;
        let ffprobe = self.ffprobe(&path)?;
        let (command, manifest) = build_ffmpeg_command(path, job.into_transcode_args(ffprobe), &outputdir, &self.url_prefix);
    }
    */

    async fn run_ffmpeg(&self, path: &Path, job: NetworkTranscodeArgs) -> Result<(), anyhow::Error> {
        info!("ffmpeg process starting: {:?}", &job.command);
        let mut proc = job.command
            .stderr(Stdio::piped())
            .spawn()
            .expect("failed to launch ffmpeg");
        let mut output_stream = tokio::io::BufReader::new(proc.stderr.take().expect("we specified stderr pipe"));
        
        let mut buf: Vec<u8> = Vec::new();
        loop {
            // XXX should this error be fatal (at least to the extent that it aborts the job)?
            let n = output_stream.read_until(b'\r', &mut buf).await?;
            debug!("ffmpeg output line: {:?}", buf.as_slice());
            if n==0 {break};
            let s = std::str::from_utf8(&buf[..n]).expect("invalid utf8 from ffprobe");
            let status_line = if let Some((output, status)) = s.rsplit_once('\n') {
                info!("ffmpeg output: {}", output);
                output_writer.write().await.push(output.to_string());
                output_notify.notify_waiters(); // method name is notify_waiters() rather than
                                                // notify_all() which would make more sense imo
                status
            } else {
                s
            };

            lazy_static! {
                static ref FFMPEG_TIME_RE: Regex = Regex::new("time=(-?[0-9]+):([0-9]{2}):([0-9]{2}\\.[0-9]+)").unwrap();
            }
            if let Some(cap) = FFMPEG_TIME_RE.captures(status_line) {
                // poor man's try/catch
                match (|| {
                    let hour: i32       = cap[1].parse()?;
                    let minute: u32     = cap[2].parse()?;
                    let mut second: f32 = cap[3].parse()?;
                    second += (hour * 3600) as f32;
                    second += (minute * 60) as f32;
                    Ok::<_, anyhow::Error>(second)}
                    )() 
                {
                    Ok(total_secs) => {let _ = progress_sender.send(total_secs);},
                    Err(e) => {warn!("ffmpeg timestamp float parsing failed: {}, ignoring", e)},
                }
            } else {
                warn!("ffmpeg status line {} did not contain a time= element.  Ignoring.", status_line);
            }
            
            buf.clear();
        }
        let retcode = proc.wait().await?;
        info!("ffmpeg completed with status code {}", retcode);
        // TODO notify clients of failure
        Ok(())
    }

    /**
     * Adds a job to the queue and returns its position in the queue,
     * for letting the user know how long they'll have to wait.
     */
    pub async fn queue_job(&self, file: &Path, args: TranscodeArgs<'_>, slug: String) -> std::io::Result<usize> {
        // TODO make this return an error if there are slashes in the slug
        let duration = args.duration;
        let output_path = self.output_prefix.join(&slug);
        std::fs::create_dir_all(&output_path)?;
        let (command, manifest) = build_ffmpeg_command(file, args, &output_path);
        // TODO do something with the manifest
        let command = command.into();
        let job = TranscodeJob {command, slug, duration};//, manifest};
        // acquire the R/W lock on the queue
        let mut queue = self.queue.write().await;
        queue.push_back(job);
        let pos = queue.len();
        std::mem::drop(queue); // release the lock
        self.queue_notify.notify_one();
        return Ok(pos);
    }

}


fn parse_ffmpeg_output(data: &str) -> (Option<&str>, Result<f32, anyhow::Error>) {
    let (output, status_line) = match data.rsplit_once('\n') {
        Some((a,b)) => (Some(a), b),
        None => (None, data),
    };
    (output, parse_ffmpeg_status(status_line))
}

fn parse_ffmpeg_status(status_line: &str) -> Result<f32, anyhow::Error> {
    lazy_static! {
        static ref FFMPEG_TIME_RE: Regex = Regex::new("time=(-?[0-9]+):([0-9]{2}):([0-9]{2}\\.[0-9]+)").unwrap();
    }
    if let Some(cap) = FFMPEG_TIME_RE.captures(status_line) {
        let hour: i32       = cap[1].parse()?;
        let minute: u32     = cap[2].parse()?;
        let mut second: f32 = cap[3].parse()?;
        second += (hour * 3600) as f32;
        second += (minute * 60) as f32;
        Ok::<_, anyhow::Error>(second)
    } else {
        Err(anyhow!("ffmpeg status line \"{}\" did not contain a time= element", status_line))?
    }
}


fn until(s: &str, until: char) -> &str {
    return s.split_once(until).map(|x|x.0).unwrap_or(s);
}

#[derive(Debug)]
pub enum BadPath {
    AttemptedDirectoryTraversal,
}

impl std::error::Error for BadPath {}
impl std::fmt::Display for BadPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use BadPath::*;
        match self {
            AttemptedDirectoryTraversal => f.write_str("Attempted direttory traversal"),
        }
    }
}


pub enum FFprobeError {
    BadPath(BadPath),
    FFprobeFailed(std::io::Error),
}

impl IntoResponse for BadPath {
    fn into_response(self) -> Response {
        use BadPath::*;
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, "text/html".parse().unwrap());
        match self {
            AttemptedDirectoryTraversal => (StatusCode::BAD_REQUEST, headers, "<h1>400 Nice Try</h1><p>You're not going to get a directory traversal exploit that easily.</p>").into_response(),
        }
    }
}

use axum::response::{Response, IntoResponse};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header::CONTENT_TYPE};
impl IntoResponse for FFprobeError {
    fn into_response(self) -> Response {
        use FFprobeError::*;
        let mut headers = HeaderMap::new();
        headers.insert(CONTENT_TYPE, HeaderValue::from_static("text/html"));
        match self {
            BadPath(e) => e.into_response(),
            FFprobeFailed(e) if e.kind() == std::io::ErrorKind::NotFound => (StatusCode::NOT_FOUND, headers, "<h1>404 Not Found</h1>").into_response(),
            FFprobeFailed(e) => (StatusCode::INTERNAL_SERVER_ERROR, headers, format!("<h1>ffprobe Error</h1><p>{}</p>", e)).into_response(),
        }
    }
}

impl From<TranscodeStatus> for cytrans_ws::TranscodeStatus {
    fn from(s: TranscodeStatus) -> Self {
        match s {
            TranscodeStatus::Idle => cytrans_ws::TranscodeStatus::Idle,
            TranscodeStatus::Working{input_file, slug, duration, ..} => cytrans_ws::TranscodeStatus::Working{input_file, slug, duration},
        }
    }
}
