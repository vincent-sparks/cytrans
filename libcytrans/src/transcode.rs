use crate::ffprobe::{FFprobeResult, Track, TrackType::*};
use crate::options::*;
use crate::codecs::BITMAP_SUBTITLE_CODECS;
use crate::metadata::*;
use std::ffi::OsStr;
use std::path::Path;
use std::process::Command;
use fixedstr::str4;
use std::collections::HashMap;
use strum::IntoEnumIterator;

#[derive(strum::EnumIter, serde::Serialize, serde::Deserialize)]
pub enum VideoContainer {
    MP4, WEBM, OGG
}


impl VideoContainer {
    fn get_acceptable_video_codecs(&self) -> &'static [VideoCodec] {
        use VideoContainer::*;
        use VideoCodec::*;
        match self {
            MP4  => &[VP9, AV1, H264, H265],
            WEBM => &[VP8, VP9, AV1],
            OGG  => &[VP8, VP9, Theora],
        }
    }

    fn get_acceptable_audio_codecs(&self) -> &'static [AudioCodec] {
        use VideoContainer::*;
        use AudioCodec::*;
        match self {
            // interesting note: Chrome supports Vorbis in MP4 files but Firefox does not.
            // Firefox will still play Vorbis if you demux it though.
            MP4  => &[Opus, AAC, ALAC, FLAC, MP3],
            WEBM => &[Opus, Vorbis],
            OGG  => &[Opus, Vorbis, FLAC],
        }
    }

    pub fn find_av(vc: VideoCodec, ac: AudioCodec) -> Option<Self> {
        Self::iter().find(|container| container.get_acceptable_video_codecs().contains(&vc) &&
               container.get_acceptable_audio_codecs().contains(&ac))
    }

    pub fn find(codec: VideoCodec) -> Self {
        use VideoContainer::*;
        use VideoCodec::*;
        match codec {
            // if we have a choice, put any codecs that can go in a webm, in a webm
            // webm files can start to play before transcoding finishes and mp4 files can't
            AV1 | VP8 | VP9 => WEBM,
            H264 | H265  => MP4,
            Theora => OGG,
        }
    }

    pub fn preferred_audio_encoder(&self) -> &'static str {
        use VideoContainer::*;
        match self {
            MP4 => "aac",
            WEBM | OGG => "libopus",
        }
    }
    pub fn extension(&self) -> &'static str {
        use VideoContainer::*;
        match self {
            MP4  => "mp4",
            WEBM => "webm",
            OGG  => "ogv",
        }
    }
    pub fn mimetype(&self) -> &'static str {
        use VideoContainer::*;
        match self {
            MP4  => "video/mp4",
            WEBM => "video/webm",
            OGG  => "video/ogg",
        }
    }
    pub fn from_extension(s: &str) -> Option<Self> {
        use VideoContainer::*;
        match s{
            "mp4"  => Some(MP4),
            "webm" => Some(WEBM),
            "ogv"  => Some(OGG),
            _      => None,
        }
    }
}

#[derive(serde::Serialize,serde::Deserialize, Clone,Copy)]
pub enum AudioContainer {
    M4A, OGG,
    // Every source I can find on the internet says that M4A files are just renamed MP4 files that
    // only contain audio tracks.  However, when I ask ffmpeg to create an M4A file and an MP4 file
    // with the exact same contents, I get two different files.  It puts the sequence "M4A_" in the
    // file subtype of one but not the other.  Also it will refuse to put any codec besides AAC or
    // ALAC in an M4A file.  To work around this, I'm producing "pseudo-M4A" files which are
    // actually literally renamed MP4 files that only contain audio tracks.  My testing says
    // browsers will still play them despite the header saying it's an ISO MP4 rather than an M4A.
    // This allows me to embed audio codecs like MP3 that cytube would otherwise reject.
    // Side note: what on Earth is Cytube's excuse for rejecting MP3 files?  At least with FLAC
    // there's a somewhat persuasive argument to be made if you haven't looked at compatibility
    // data.
    PseudoM4A,
}


impl AudioContainer {
    pub fn extension(&self) -> &'static str {
        use AudioContainer::*;
        match self {
            OGG => "ogg",
            M4A | PseudoM4A => "m4a",
        }
    }
    pub fn mimetype(&self) -> &'static str {
        use AudioContainer::*;
        match self {
            OGG => "audio/ogg",
            M4A | PseudoM4A => "audio/mp4",
        }
    }
    pub fn find(codec: AudioCodec) -> AudioContainer {
        // Now here's where things get wacky.
        // Cytube doesn't support adding bare FLAC files, citing browser compatiblitity
        // issues with the FLAC codec.
        // Maybe the documentation is just old and Cytube hasn't been updated in a while,
        // but caniuse.com tells a very different story: green lights across the board for
        // any browser released in the last couple years, with a 95% compatibility rating.
        // I should probably see about bugging the guys at Cytube to remove that
        // restriction.
        // In the meantime, however, just because we can't use the FLAC *container*
        // doesn't mean that we can't play FLAC-encoded *audio*.
        // You see, one of the container formats that Cytube *does* accept is Ogg, and
        // there are three audio codecs (that browsers support) that can go inside an
        // Ogg file: Vorbis, Opus, and FLAC.
        // If we embed FLAC data inside an Ogg file, Cytube won't know the difference.  The
        // entire point of the custom metadata files is that Cytube doesn't have to
        // retrieve the files from the media host to run ffprobe on them.  It doesn't know
        // about the codecs, only the container.  We just tell the server we have an
        // Ogg file and it says "great" and ships it to the clients.
        // The Cytube client (webpage) doesn't do any enforcement on its end.  As long as
        // the browser can play it, it'll play ball.
        // We can play FLAC files, we just can't *tell Cytube* we're playing FLAC files.
        use AudioContainer::*;
        use AudioCodec::*;
        match codec {
            AAC  | ALAC => M4A,
            Opus | Vorbis | FLAC=> OGG,
            // cytube doesn't support MP3 for some reason
            // fortunately we can use the same trick we use with flac
            MP3 => PseudoM4A,
        }
    }
}



pub fn get_defaults<'a>(ffprobe: &'a FFprobeResult, file: &Path) -> TranscodeArgs<'a> {
    let mut subtitle_tracks: Vec<&Track> = Vec::new();
    let mut audio_tracks: Vec<&Track> = Vec::new();
    let mut video_tracks: Vec<&Track> = Vec::new();

    let mut subtitle_reqs: Vec<&Track> = Vec::new();
    let mut audio_reqs: Vec<TrackOptions<AudioCodec>> = Vec::new();
    let mut video_reqs: Vec<TrackOptions<VideoCodec>> = Vec::new();

    for track in &ffprobe.tracks {
        match track.kind {
            Video => video_tracks.push(track),
            Audio => audio_tracks.push(track),
            Subtitle => subtitle_tracks.push(track),
        }
    }

    let video_codec = video_tracks.first().map(|track| {
        let codec: Option<VideoCodec> = track.codec.parse().ok();
        let encoder = if codec.is_some() {"copy"} else {"libsvtav1"}.to_string();
        let codec = codec.unwrap_or(VideoCodec::AV1);
        video_reqs.push(TrackOptions {
            track,
            codec,
            encoder,
            bitrate: None,
            extra_ffmpeg_args: vec![],
        });
        codec
    });
    
    // if there's a way to do this idiomatically and declaratively i'd love to hear about it
    let mut audio_tracks_by_language = HashMap::<str4, Vec<&Track>>::new();
    for track in audio_tracks.iter() {
        audio_tracks_by_language.entry(track.language.unwrap_or("".into()))
            .or_default()
            .push(*track);
    }
    let chosen_tracks = if audio_tracks_by_language.len() == 1 {
        let tracks = audio_tracks_by_language.values().next().unwrap();
        let track = tracks.iter().max_by_key(|track| {
            let mut score = 0;
            if let Ok(ac) = track.codec.parse::<AudioCodec>() {
                score += 100;
                if let Some(vc) = video_codec {
                    if VideoContainer::find_av(vc, ac).is_some() {
                        score += 100;
                    }
                }
            }
            score + track.channels.unwrap_or(0)
        }).unwrap();
        vec![*track] // get rid of the double reference
    } else {
        // this line is about as readable as a python list comprehension so let me explain.
        // we've already grouped the audio tracks by language. we select one track per
        // language to go into the final chosen list, and we decide which one based first on
        // whether or not it will fit into a web format without transocding, then by the number of
        // channels it has.
        audio_tracks_by_language.values().map(|x| *x.iter().max_by_key(|track| if track.codec.parse::<AudioCodec>().is_ok() {100} else {0} + track.channels.unwrap_or(0)).unwrap()).collect()
    };
    for track in chosen_tracks {
        let (codec, encoder) = if let Ok(x) = track.codec.parse() {
            (x, "copy".to_string())
        } else {
            if let Some(vc) = video_codec {
                if matches!(VideoContainer::find(vc), VideoContainer::MP4) {
                    (AudioCodec::AAC, "aac".to_string())
                } else {
                    (AudioCodec::Opus, "libopus".to_string())
                }
            } else {
                (AudioCodec::AAC, "aac".to_string())
            }
        };
        audio_reqs.push(TrackOptions {
            track, codec, encoder,
            bitrate: None,
            extra_ffmpeg_args: vec![],
        });
    }

    // put all subtitle tracks by default, except the bitmap ones
    for track in subtitle_tracks {
        if !BITMAP_SUBTITLE_CODECS.contains(&track.codec.as_str()) {
            // some subtitle formats store the subtitles as transparent images that get
            // composited onto the video rather than as text, so that e.g. DVD players don't have
            // to store fonts with every possible non-Latin symbol that they could need in their
            // firmware.
            //
            // unfortunately the only subtitle format Cytube accepts is vtt, which is a text format,
            // and although ffmpeg can convert between text formats, it can't do OCR, so unless
            // we want to burn the subtitles into the video track, we can't use them.
            //
            // the ffprobe code does return such tracks to the user in the event that they do in
            // fact want to do that, but we must check here in the defaults selection and in any
            // client that allows selecting subtitle tracks that any track we pass to cytube must
            // not have a codec in this list.  (I won't bother to perform this check in the
            // server-side code, since ffmpeg will do it for me)
            subtitle_reqs.push(track);
        }
    }

    TranscodeArgs {
        video_tracks: video_reqs,
        audio_tracks: audio_reqs,
        subtitle_tracks: subtitle_reqs,
        duration: ffprobe.duration,
        title: ffprobe.title.to_owned().unwrap_or_else(|| file.file_stem().unwrap_or(file.as_os_str()).to_string_lossy().to_string()),
        extra_ffmpeg_args: Vec::new(),
        force_demux_audio: false,
        add_muxed_silence: false,
    }
}



pub fn build_ffmpeg_command(media_file: &OsStr,
                            transcode_args: TranscodeArgs,
                            outputdir: &Path) -> (Command, MetadataManifest, bool) {
    let mut command = Command::new("ffmpeg");
    command.arg("-hide_banner");
    command.args(transcode_args.extra_ffmpeg_args);
    command.arg("-i").arg(media_file);

    let mut video_out = Vec::new();
    let mut audio_out = Vec::new();
    let mut text_out = Vec::new();

    let mut will_demux_audio = transcode_args.force_demux_audio;

    if !will_demux_audio {
        // if there is more than one audio track, we must demux.
        // If the video file contains an audio track, and supplemental tracks are provided, the
        // browser will show the dropdown for the supplemental tracks, but will play the selected
        // one simultaneously with the track muxed into the video file.
        // I found this out by listening to the English and Japanese dubs of an anime
        // simultaneously.
        // We also set will_demux_audio to true if there are 0 audio tracks, to tell the code below
        // that it shouldn't mux an audio track into the video file.
        if transcode_args.audio_tracks.len() != 1 {
            will_demux_audio = true;
        } else {
            // if we're going to use ANY video codecs that can't fit in the same container as our
            // chosen audio codec, demux.
            let audio_codec = transcode_args.audio_tracks[0].codec;
            for track in transcode_args.video_tracks.iter() {
                if VideoContainer::find_av(track.codec, audio_codec).is_none() {
                    will_demux_audio = true;
                    break;
                }
            }
        }
    }

    let (muxed_audio_track, muxed_audio_idx) = if !will_demux_audio {
        let ref track = transcode_args.audio_tracks[0]; // syntactic sugar
        (Some(track), Some(format!("0:{}", track.track.index)))
    } else if transcode_args.add_muxed_silence {
        // generate a silent audio track to work around a quirk in some browsers when playing
        // demuxed video, where it will stop the audio playing when you switch tabs because you
        // can't see the video anymore.  Cytube docs recommended I do this, but they haven't been
        // updated in years so I doubt if it's really still necessary.  I implemented it anyway and
        // made it optional, just to be safe.
        // TODO copy the sample rate and channel layout from the source file!
        command.args(["-f", "lavfi", "-t", transcode_args.duration.to_string().as_str(), "-i", "anullsrc=channel_layout=stereo:sample_rate=48000",
        ]);
        (None, Some("1:0".to_string()))
    } else {(None, None)};
    
    for video in transcode_args.video_tracks {
        command.args(["-map", format!("0:{}", video.track.index).as_str()]); 
        if let Some(ref idx) = muxed_audio_idx {
            command.args(["-map", idx.as_str()]);
        }

        let video_container = if let Some(audio) = muxed_audio_track {

            // this unwrap is safe because the above code already contains a check for whether there
            // are any video tracks that cannot share a container with this codec.
            let container = VideoContainer::find_av(video.codec, audio.codec).unwrap();
            if matches!(audio.codec, AudioCodec::FLAC) && matches!(container, VideoContainer::MP4) {
                // ffmpeg doesn't like putting FLAC streams inside MP4 files, considers it
                // experimental.  we have to tell it that that's okay
                // for some reson ffmpeg mandates this be done on a per-output-file basis
                command.args(["-strict", "experimental"]);
            }
            container
        } else {
            VideoContainer::find(video.codec)
        };

        let filename = if video.encoder == "copy" {
            format!("main.{}", video_container.extension())
        } else {
            format!("video{}_{}.{}", video.track.index, video.codec.as_ref(), video_container.extension())
        };

        let encoder: &str = if video.encoder != "" {
            video.encoder.as_str()
        } else {
            video.codec.as_ref()
        };
        command.args(["-c:v", encoder]); 
        if let Some(audio) = muxed_audio_track {
            let encoder: &str = if audio.encoder != "" {
                audio.encoder.as_str()
            } else {
                audio.codec.as_ref()
            };
            command.args(["-c:a", encoder]);
            command.args(&audio.extra_ffmpeg_args);
        }

        command.args(video.extra_ffmpeg_args);
        command.arg(outputdir.join(&filename));

        let resolution_h = video.track.resolution_h.unwrap_or(0);
        let resolution_v = video.track.resolution_v.unwrap_or(0);

        video_out.push(VideoMetadata {
            filename,
            audio_is_silent: transcode_args.add_muxed_silence,
            container: video_container,
            video_codec: video.codec,
            audio_codec: muxed_audio_track.map(|x|x.codec),
            resolution_h,
            resolution_v,
        });
    }

    let muxed_audio;
    if will_demux_audio {
        for audio in transcode_args.audio_tracks {
            let container = AudioContainer::find(audio.codec);
            let encoder: &str = if audio.encoder != "" {
                audio.encoder.as_str()
            } else {
                audio.codec.as_ref()
            };
            command.args([
                         "-map",
                         format!("0:{}", audio.track.index).as_str(),
                         "-codec",
                         encoder,
            ]);
            command.args(audio.extra_ffmpeg_args);
            let language = audio.track.language.unwrap_or("unk".into());
            let filename = format!("audio_{}_{}.{}", audio.track.index, audio.track.language.as_ref().map(|x| x.as_str()).unwrap_or("unknown"), container.extension());
            if matches!(&container, AudioContainer::PseudoM4A) {
                command.args(["-f", "mp4"]);
            }
            command.arg(outputdir.join(&filename));
            audio_out.push(AudioMetadata {
                codec: audio.codec,
                filename,
                container,
                language,
                title: audio.track.title.to_owned(),
            });
        }
        muxed_audio = None;
    } else {
        let audio = transcode_args.audio_tracks.iter().next().unwrap();
        muxed_audio = Some(MuxedAudioMetadata {language: audio.track.language.to_owned().unwrap_or("unk".into()), title: audio.track.title.to_owned()});
    }

    for sub_track in transcode_args.subtitle_tracks {
        command.args(["-map", format!("0:{}", sub_track.index).as_str()]);
        let lang = match &sub_track.language {
            Some(x) => x.as_str(),
            None => "unknown",
        };
        let filename = format!("sub_{}_{}.vtt", sub_track.index, lang);
        command.arg(outputdir.join(&filename).as_os_str());

        text_out.push(TextMetadata {
            filename,
            language: sub_track.language,
            title: sub_track.title.to_owned(),
        });
    }

    dbg!(&command);
    (command, MetadataManifest {
        title: transcode_args.title,
        duration: transcode_args.duration,
        video_files: video_out,
        audio_files: audio_out,
        text_files: text_out,
        muxed_audio, 
    }, will_demux_audio)
}

pub fn build_demux_commands(mut meta: MetadataManifest, root_path: &Path) -> (Vec<Command>, MetadataManifest) {
    /*
     * This function makes the following assumptions and may misbehave if they are not upheld:
     *  1. The metadata in `meta` is accurate and ffprobe does not need to be run.
     *  2. `meta` describes at least one video file.
     *  3. Each video file has exactly one video track and at most one audio track.
     *  4. Only one audio track will need to be demuxed.
     */
    let mut commands = Vec::new();

    if let Some(muxed_audio_meta) = meta.muxed_audio.take() {
        // TODO account for bitrate when choosing an audio track to demux.
        let Some(best) = meta.video_files.iter().filter(|x| x.audio_codec.is_some() && !x.audio_is_silent)
            .max_by_key(|x| (x.audio_codec == Some(AudioCodec::FLAC), )) else {
                // no videos have an audio track -- nothing to do
                return (vec![], meta)
            };

        let codec = best.audio_codec.unwrap();
        let container = AudioContainer::find(codec);

        let audio_track = AudioMetadata {
            codec,
            container,
            filename: format!("demuxed.{}", container.extension()),
            language: muxed_audio_meta.language,
            title: muxed_audio_meta.title,
        };

        // rust doesn't like it when we borrow the same value as mutable and immutable at the same
        // time.  we coerce the (checked) borrow to an (unchecked) pointer to force it to let us do
        // that.  do NOT try this at home.
        let best: *const VideoMetadata = best;

        for vid in meta.video_files.iter_mut() {
            if vid.audio_codec.is_some() && !vid.audio_is_silent {
                let mut command = Command::new("ffmpeg");
                let container = VideoContainer::find(vid.video_codec);
                let (name, _ext) = vid.filename.rsplit_once('.').unwrap();
                let new_name = format!("{}_demuxed.{}", name, container.extension());
                command.arg("-i");
                command.arg(root_path.join(&vid.filename));
                command.args(["-an","-c:v","copy"]);
                command.arg(root_path.join(&new_name));
                vid.filename=new_name;

                if std::ptr::eq(vid, best) {
                    command.args(["-vn", "-c:a", "copy"]);
                    command.arg(root_path.join(&audio_track.filename));
                }
                commands.push(command);
            }
        }

        meta.audio_files.push(audio_track);
    }

    (commands, meta)
}
