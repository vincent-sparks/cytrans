use std::path::Path;
use std::process::{Command, Stdio};
use fixedstr::str4;
use serde::{Serialize,Deserialize};

#[derive(Debug)]
#[derive(strum::EnumString, Serialize, Deserialize, Clone)]
#[strum(serialize_all="snake_case")]
pub enum TrackType {
    Video,
    Audio,
    Subtitle,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Track {
    pub index: u16,
    pub kind: TrackType,
    pub codec: String,
    pub scanline_count: Option<u16>,
    pub language: Option<str4>,
    pub title: Option<String>,
    pub channels: Option<u8>,
}

impl std::fmt::Display for Track {
    fn fmt(&self, fmt:&mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "#{}", self.index)?;
        if let Some(title) = &self.title {
            write!(fmt, " \"{}\"", title)?;
        }
        if let Some(lang) = &self.language {
            write!(fmt, " ({})", lang)?;
        }
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FFprobeResult {
    pub tracks: Vec<Track>,
    pub title: Option<String>,
    pub duration: f32,
    pub bitrate: u64, // in kbps
}

fn parse_ffmpeg_line<'a>(line: &'a str) -> (&'a str, impl Iterator<Item=(&'a str, &'a str)>) {
    let mut it = line.split("|");
    let kind = it.next().unwrap();
    return (kind, it.map(|token| token.split_once("=").unwrap()));
}

//#[cfg(feature="commands")]
pub fn ffprobe(filename: &Path) -> std::io::Result<FFprobeResult> {
    filename.metadata()?; // to make sure we can read the path before invoking ffmpeg
                          // you could remove this but it would make error messages less
                          // informative
    let res = Command::new("ffprobe")
        .arg(filename.as_os_str())
        .arg("-of").arg("compact")
        .arg("-hide_banner")
        .arg("-show_streams").arg("-show_format")
        .arg("-show_entries")
        .arg("stream_tags=title,language:stream=index,codec_type,codec_name,channels,coded_height:stream_disposition=:format=duration,bit_rate:format_tags=title")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?
        .wait_with_output()?;
    if !res.status.success() {
        let mut s = String::from("ffprobe returned error: ");
        s.push_str(std::str::from_utf8(&res.stderr).expect("ffmpeg returned invalid utf8"));
        return Err(std::io::Error::new(std::io::ErrorKind::Other, s));
    }
    let output = std::str::from_utf8(&res.stdout).unwrap();
    let mut tracks = Vec::<Track>::new();
    let mut title: Option<String> = None;
    let mut duration = 0.0f32;
    let mut bitrate = 0u64;

    'a: for line in output.split("\n") {
        let (kind, params) = parse_ffmpeg_line(line);
        match kind {
            "format" => {
                for (k,v) in params {
                    match k {
                        "duration" => {duration = v.parse().unwrap();}
                        "bit_rate" => {bitrate = v.parse().unwrap();}
                        "tag:title" => {title = Some(v.to_owned());}
                        x => {println!("uncrecognized tag {}", x);},
                    }
                }
            },
            "stream" => {
                let mut kind: Option<TrackType> = None;
                let mut codec: Option<String> = None;
                let mut scanline_count: Option<u16> = None;
                let mut language: Option<str4> = None;
                let mut title: Option<String> = None;
                let mut index: Option<u16> = None;
                let mut channels: Option<u8> = None;
                for (k,v) in params {
                    match k {
                        "codec_type" => {
                            kind = Some(match v.parse() {
                                Ok(x) => x,
                                Err(_) => continue 'a, // not a track type we're interested in
                            });
                        },
                        "index" => index = Some(v.parse().unwrap()),
                        "channels" => channels = Some(v.parse().unwrap()),
                        "codec_name" => codec = Some(v.to_string()),
                        "coded_height" => scanline_count = Some(v.parse().unwrap()),
                        "tag:language" => language = Some(v.into()),
                        "tag:title" => title = Some(v.to_string()),
                        x => {println!("ffprobe returned uncrecognized tag {}", x);},
                    }
                }
                let index = index.expect("no index");
                let kind = kind.expect("no codec_type");
                let codec = codec.expect("no codec_name");
                tracks.push(Track {index, kind, codec, scanline_count, language, title, channels});
            },
            _ => {},
        }
    }
    Ok(FFprobeResult {tracks, title, duration, bitrate})
}

