use serde::{Serialize,Deserialize};

#[derive(Serialize, Deserialize)]
pub enum TranscodeStatus {
    Idle, Working {
        input_file: String,
        slug: String,
        duration: f32,
    }
}

#[derive(Serialize, Deserialize)]
pub enum WebsocketMessage {
    StatusUpdate(TranscodeStatus),
    Progress(f32),
}

#[derive(Serialize)] // deserialization is performed by custom code on the server
pub struct TranscodeRequest<'ff> {
    pub args: cytrans::options::TranscodeArgs<'ff>,
    pub slug: String,
}

#[derive(serde::Deserialize)]
pub struct NetworkTrackOptions<C> {
    pub track_idx: u16,
    pub codec: C,
    pub extra_ffmpeg_args: Vec<String>,
    pub encoder: String,
    pub bitrate: Option<u32>,
}

#[derive(serde::Deserialize)]
pub struct NetworkTranscodeArgs {
    pub video_tracks: Vec<NetworkTrackOptions<cytrans::options::VideoCodec>>,
    pub audio_tracks: Vec<NetworkTrackOptions<cytrans::options::AudioCodec>>,
    pub subtitle_tracks: Vec<u16>,
    pub extra_ffmpeg_args: Vec<String>,
    pub title: String,
    pub slug: String
}
