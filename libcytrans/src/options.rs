use crate::ffprobe::Track;
use std::ffi::OsString;
use std::path::Path;
use serde::{Serialize, Serializer, ser::SerializeSeq};

use std::fmt::*;

#[derive(Debug, PartialEq, Clone, Copy, strum::EnumString, strum::EnumIter, strum::AsRefStr, serde::Serialize, serde::Deserialize)]
#[strum(serialize_all="snake_case")]
pub enum VideoCodec {
    AV1, VP8, VP9,
    H264,
    #[strum(serialize="hevc")] 
    H265,
    Theora,
}

// Strum's Display logic kinda sucks
// and I don't really feel like vendoring *another* crate right now
// so i have to do this manually. *sigh*
impl Display for VideoCodec {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result {
        use VideoCodec::*;
        fmt.write_str(match self {
            AV1 => "AV1",
            VP8 => "VP8",
            VP9 => "VP9",
            H264 => "H.264",
            H265 => "H.265",
            Theora => "Theora",
        })
    }
}


#[derive(Debug, PartialEq, Clone, Copy, strum::EnumString, strum::EnumIter, strum::AsRefStr, serde::Serialize, serde::Deserialize)]
#[strum(serialize_all="snake_case")]
pub enum AudioCodec {
    AAC,
    #[strum(serialize="alac", serialize="alac_latm")]
    ALAC,
    Opus,
    Vorbis,
    FLAC,
    MP3,
}

impl Display for AudioCodec {
    fn fmt(&self, fmt: &mut Formatter<'_>) -> Result {
        use AudioCodec::*;
        fmt.write_str(match self {
            AAC => "AAC",
            ALAC => "ALAC",
            Opus => "Opus",
            Vorbis => "Vorbis",
            FLAC => "FLAC",
            MP3 => "MP3",
        })
    }
}

#[derive(Clone, Serialize)]
pub struct TrackOptions<'a, C> {
    #[serde(serialize_with="serialize_track_id")]
    pub track: &'a Track,
    // Debating on whether I want to have this parameter
    //pub output_filename: &'b Path,
    pub codec: C,
    pub extra_ffmpeg_args: Vec<OsString>,
    pub encoder: String,
    pub bitrate: Option<u32>,
}

#[derive(Serialize)]
pub struct TranscodeArgs<'ff> {
    pub video_tracks: Vec<TrackOptions<'ff, VideoCodec>>,
    pub audio_tracks: Vec<TrackOptions<'ff, AudioCodec>>,
    #[serde(serialize_with="serialize_track_ids")]
    pub subtitle_tracks: Vec<&'ff Track>,
    pub extra_ffmpeg_args: Vec<OsString>,
    pub title: String,
    #[serde(skip_serializing)]
    pub duration: f32,
    pub force_demux_audio: bool,
    pub add_muxed_silence: bool,
}

fn serialize_track_id<S: serde::Serializer>(track: &Track, s: S) -> std::result::Result<S::Ok, S::Error> {
    s.serialize_u16(track.index)
}

fn serialize_track_ids<S: serde::Serializer>(tracks: &Vec<&Track>, s: S) -> std::result::Result<S::Ok, S::Error> {
    let mut a = s.serialize_seq(Some(tracks.len()))?;
    for track in tracks {
        a.serialize_element(&track.index)?;
    }
    a.end()
}
