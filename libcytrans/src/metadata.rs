use crate::options::{VideoCodec, AudioCodec};
use crate::ffmpeg_languages::{LANGUAGES, FF2CT};
use crate::cytube_structs as cytube;
use crate::transcode::{VideoContainer, AudioContainer};
use serde::{Serialize,Deserialize};

const CYTUBE_QUALITY_VALUES: [u16; 7] = [240, 360, 480, 540, 720, 1080, 2160];

#[derive(Serialize,Deserialize)]
pub struct VideoMetadata {
    pub filename: String,
    pub container: VideoContainer,
    pub video_codec: VideoCodec,
    pub audio_codec: Option<AudioCodec>,
    pub audio_is_silent: bool,
    pub resolution_h: u16,
    pub resolution_v: u16,
}

#[derive(Serialize,Deserialize)]
pub struct AudioMetadata {
    pub filename: String,
    pub container: AudioContainer,
    pub codec: AudioCodec,
    pub language: fixedstr::str4,
    pub title: Option<String>,
}

#[derive(Serialize,Deserialize)]
pub struct TextMetadata {
    pub filename: String,
    pub language: Option<fixedstr::str4>,
    pub title: Option<String>,
}

#[derive(Serialize,Deserialize)]
pub struct MetadataManifest {
    pub video_files: Vec<VideoMetadata>,
    pub audio_files: Vec<AudioMetadata>,
    pub text_files:  Vec<TextMetadata>,
    pub duration: f32,
    pub title: String,
    pub muxed_audio: Option<MuxedAudioMetadata>,
}

/**
 * Metadata about the audio track muxed into the video.  Used when demultiplexing.
 */
#[derive(Serialize,Deserialize)]
pub struct MuxedAudioMetadata {
    pub language: fixedstr::str4,
    pub title: Option<String>,
}

#[derive(Serialize,Deserialize)]
pub struct ToRemove {
    pub video_files: Vec<String>,
    pub audio_files: Vec<String>,
    pub text_files: Vec<String>,
}

fn strcat(first: &str, rest: &str) -> String {
    let mut s = String::from(first);
    s.push_str(rest);
    s
}

fn build_language_string(language: &str, title: Option<&str>) -> String {
    let mut s = String::from(*LANGUAGES.get(language).unwrap_or(&language));
    if let Some(title) = title {
        s.push_str(" (");
        s.push_str(title);
        s.push(')');
    }
    s
}

fn snap_to_nearest(val: u16, legal: &[u16]) -> u16 {
    let mut last_difference = u16::MAX;
    for (i,a) in legal.iter().copied().enumerate() {
        if a > val {
            if a - val > last_difference {
                return legal[i-1];
            } else {
                return a;
            }
        }
        last_difference = val - a;
    }
    return legal[legal.len()-1];
}

impl VideoMetadata {
    pub fn to_source(&self, url_prefix: &str) -> cytube::Source {
        cytube::Source {
            bitrate: None,
            quality: snap_to_nearest(self.resolution_v, &CYTUBE_QUALITY_VALUES),
            content_type: self.container.mimetype(),
            url: strcat(url_prefix, &self.filename),
        }
    }
}

impl AudioMetadata {
    pub fn to_source(&self, url_prefix: &str) -> cytube::Source {
        cytube::Source {
            bitrate: None,
            quality: 240,
            content_type: self.container.mimetype(),
            url: strcat(url_prefix, &self.filename),
        }
    }
    pub fn to_audio_track(&self, url_prefix: &str) -> cytube::AudioTrack {
        let ref language = self.language.as_str();
        cytube::AudioTrack {
            content_type: self.container.mimetype(),
            language: FF2CT.get(language).unwrap_or(language).to_string(),
            url: strcat(url_prefix, &self.filename),
            label: build_language_string(&language, self.title.as_ref().map(|x|x.as_str())),
        }
    }
}

impl TextMetadata {
    pub fn to_text_track(&self, url_prefix: &str) -> cytube::TextTrack {
        let language_string = match self.language {
            Some(x) => build_language_string(x.as_str(), self.title.as_ref().map(|x|x.as_str())),
            None => self.title.clone().unwrap_or("Unknown".to_string()),
        };
        cytube::TextTrack {
            content_type: "text/vtt",
            url: strcat(url_prefix, self.filename.as_str()),
            name: language_string,
        }
    }
}

impl MetadataManifest {
    pub fn to_cytube(&self, url_prefix: &str) -> cytube::CytubeVideo {
        cytube::CytubeVideo {
            title: self.title.clone(),
            duration: self.duration,
            sources: self.video_files.iter().map(|x| x.to_source(url_prefix)).collect(),
            audio_tracks: self.audio_files.iter().map(|x| x.to_audio_track(url_prefix)).collect(),
            text_tracks: self.text_files.iter().map(|x| x.to_text_track(url_prefix)).collect(),
        }
    }

    pub fn discard(&mut self, discard: &ToRemove) {
        self.video_files.retain(|x| !discard.video_files.contains(&x.filename));
        self.audio_files.retain(|x| !discard.audio_files.contains(&x.filename));
        self.text_files.retain(|x| !discard.text_files.contains(&x.filename));
    }
}


