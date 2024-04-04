use std::process::{Command, Stdio};
use crate::options::{AudioCodec, VideoCodec};
use once_cell::sync::Lazy;
use strum::IntoEnumIterator;

pub const BITMAP_SUBTITLE_CODECS: [&'static str; 4] = [
    "dvb_subtitle",
    "dvd_subtitle",
    "hdmv_pgs_subtitle",
    "xsub",
];

pub type Capabilities = (Vec<(VideoCodec, Vec<String>)>, Vec<(AudioCodec, Vec<String>)>);

static FFMPEG_OUTPUT: Lazy<Capabilities> = Lazy::new(|| {
let out = Command::new("ffmpeg").args(["-hide_banner", "-codecs"])
        .stdout(Stdio::piped())
        .spawn().expect("error launching ffmpeg")
        .wait_with_output().expect("error waiting for ffmpeg");
        assert!(out.status.success(), "ffmpeg -codecs returned error");
        let ffmpeg_output = std::str::from_utf8(&out.stdout).expect("ffmpeg output wasn't utf8");
        (get_encoder_names(ffmpeg_output, VideoCodec::iter().collect()),
         get_encoder_names(ffmpeg_output, AudioCodec::iter().collect()))
});

pub fn get_video_encoders() -> &'static Vec<(VideoCodec, Vec<String>)> {
    &FFMPEG_OUTPUT.0
}
pub fn get_audio_encoders() -> &'static Vec<(AudioCodec, Vec<String>)> {
    &FFMPEG_OUTPUT.1
}

pub fn get_capabilities() -> &'static Capabilities {
    &FFMPEG_OUTPUT
}

pub fn get_encoder_names<T: AsRef<str>>(ffmpeg_output: &str, mut codec_names: Vec<T>) -> Vec<(T, Vec<String>)> {

    // this doesn't work
    // rust pls fix
    /*
    let mut codec_names2: HashMap<&str, T> = HashMap::new();
    for t in codec_names {
        codec_names2.insert(t.as_ref(), t);
    }
    */
    let mut result = Vec::new();

    let lines = ffmpeg_output.lines();

    'a: for line in lines {
        if line.as_bytes()[2] != 'E' as u8 {
            // ffmpeg can't encode this codec
            continue;
        }
        let line = line.get(8..).unwrap();
        let tab_idx = line.find(' ').unwrap();
        let codec_name = line.get(0..tab_idx).unwrap();
        let idx = 'b: {
            for (i,name) in codec_names.iter().enumerate() {
                if name.as_ref() == codec_name {
                    break 'b i;
                }
            }
            continue 'a;
        };
        let codec = codec_names.swap_remove(idx);
        let line = line.get(tab_idx..).unwrap();
        
        if let Some(idx) = line.find("(encoders: ") {
            let line = line.get(idx+11..).unwrap();
            let line = line.get(..line.find(')').unwrap()).unwrap();
            let mut res = line.split(' ').collect::<Vec<_>>();
            debug_assert!(res[res.len()-1]=="");
            res.remove(res.len()-1);
            result.push((codec, res.iter().map(|x|x.to_string()).collect::<Vec<_>>()));
        } else {
            result.push((codec, vec![]));
        }
    }

    result
}


/*
pub fn get_encoder_names(typ: char) -> HashMap<String, Vec<String>> {

    let mut result = HashMap::new();

    let res = Command::new("ffmpeg").args(["-hide_banner", "-codecs"])
        .stdout(Stdio::piped())
        .spawn().expect("error launching ffmpeg")
        .wait_with_output().expect("error waiting for ffmpeg");
    let lines = std::str::from_utf8(&res.stdout).expect("error parsing utf8").lines();

    for line in lines {
        if line.as_bytes()[2] != 'E' as u8 || line.as_bytes()[3] != typ as u8 {
            // ffmpeg can't encode this codec, or it's not the type we're interested in
            continue;
        }
        let line = line.get(8..).unwrap();
        let mut split = line.split_ascii_whitespace();
        let codec_name = split.next().unwrap().to_string();
        //let line = split.as_str();
        
        if let Some(idx) = line.find("(encoders: ") {
            let line = line.get(idx+11..).unwrap();
            let line = line.get(..line.find(')').unwrap()).unwrap();
            let mut res = line.split(' ').collect::<Vec<_>>();
            debug_assert!(res[res.len()-1]=="");
            res.remove(res.len()-1);
            result.insert(codec_name, res.iter().map(|x|x.to_string()).collect::<Vec<_>>());
        } else {
            result.insert(codec_name, vec![]);
        }
    }

    result
}
*/
