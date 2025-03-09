use std::{ffi::OsString, fmt::Display, path::PathBuf, str::FromStr};

use clap::Parser;
use console_menu::{Menu, MenuOption, MenuProps};
use cytrans::{codecs::get_capabilities, ffprobe::{ffprobe, Track, TrackType}, options::{TrackOptions, TranscodeArgs, VideoCodec}};

#[derive(clap::Parser)]
#[command(version, about)]
struct Args {
    input_path_or_url: OsString,
    output_directory: PathBuf,
    url_prefix: String,
}

#[derive(strum::EnumMessage, strum::EnumIter)]
enum MainMenuAction {
    #[strum(message="Video tracks")]
    VideoTracks,
    #[strum(message="Audio tracks")]
    AudioTracks,
    #[strum(message="Title")]
    Title,
    #[strum(message="Done, launch ffmpeg")]
    Go,
}

trait EnumMenu {
    fn into_menu() -> Vec<MenuOption<Self>> where Self: Sized;
}

impl<T> EnumMenu for T where T: strum::EnumMessage + strum::IntoEnumIterator {
    fn into_menu() -> Vec<MenuOption<Self>> where Self: Sized {
        Self::iter()
            .map(|variant| MenuOption {
                label: variant.get_message().unwrap().into(),
                value: variant,
            })
            .collect()
    }
}

fn main() {
    let args = Args::parse();

    let ffprobe_result = ffprobe(&args.input_path_or_url).expect("Error running ffprobe");

    let video_tracks = ffprobe_result.tracks.iter().filter(|track| track.kind == TrackType::Video).collect::<Vec<_>>();

    let video_track = match video_tracks.len() {
        0 => None,
        1 => Some(video_tracks[0]),
        _ => {
            let mut menu = console_menu::Menu::new(
                video_tracks
                .iter()
                .enumerate()
                .map(|(i, track)| MenuOption {
                    label: format!("[{}] \"{}\" ({}, {}p)", track.index, track.title.as_deref().unwrap_or_default(), track.codec, track.scanline_count.unwrap_or(0)),
                    value: i,
                })
                .collect(),
                MenuProps {
                    title: "Choose a video track",
                    message: "The selected file has multiple video tracks.  You can only transcode one.",
                    ..MenuProps::default()
                }
            );
            let idx = menu.show().expect("You must select a video track");
            Some(video_tracks[*idx])
        },
    };
    let mut video_tracks = Vec::new();
    let mut audio_tracks = Vec::new();


    if let Some(ref video) = video_track {
        if let Some((codec, encoder)) = choose_encoder("Choose video encoder", cytrans::codecs::get_video_encoders(), Some(&video.codec)) {
            video_tracks.push(TrackOptions {
                track: video,
                codec,
                encoder: encoder.into(),
                extra_ffmpeg_args: Vec::new(),
                bitrate: None,
            });
        }
    }

    let mut title = ffprobe_result.title.clone().unwrap_or_else(|| todo!());
    let extra_ffmpeg_args = Vec::new(); // TODO add a way to specify extra ffmpeg args
    


    let mut main_menu = Menu::new(
        MainMenuAction::into_menu(),
        MenuProps {
            title: "Main menu",
            ..MenuProps::default()
        }
    );

    loop {
        match main_menu.show() {
            Some(MainMenuAction::VideoTracks) => {
                
            },
            Some(MainMenuAction::AudioTracks) => {

            },
            Some(MainMenuAction::Title) => {

            },
            Some(MainMenuAction::Go) => break,
            None => {
                println!("User exited from main menu, not running ffmpeg.");
                return;
            },
        }
    }

    let transcode_args = TranscodeArgs {
        video_tracks, audio_tracks, title,
        subtitle_tracks: ffprobe_result.tracks.iter().filter(|x| x.is_valid_subtitle_track()).collect(),
        extra_ffmpeg_args,
        duration: ffprobe_result.duration,
        force_demux_audio: false,
        add_muxed_silence: false,
    };

    // TODO: implement invoking ffmpeg
}

fn choose_encoder<T: Copy + Display + FromStr + Into<&'static str>>(title: &str, choices: &'static [(T, Vec<String>)], origin_codec: Option<&str>) -> Option<(T, &'static str)> {
    let mut v = Vec::new();
    if let Some(origin_codec) = origin_codec {
        if let Ok(codec) = T::from_str(origin_codec) {
            v.push(MenuOption {label: format!("{} (copy)", codec), value: (codec, "copy")});
        }
    }
    for (codec, encoders) in choices {
        let codec = *codec;
        if encoders.is_empty() {
            v.push(MenuOption {label: codec.to_string(), value: (codec, codec.into())});
        } else {
            for encoder in encoders {
                v.push(MenuOption {label: format!("{} ({})", codec, encoder), value: (codec, encoder.as_str())});
            }
        }
    }
    let mut menu = Menu::new(v, MenuProps {
        title,
        ..MenuProps::default()
    });
    menu.show().copied()
}

trait Menuable<'ff>: Sized {
    const TRACK_TYPE: TrackType;
    const MENU_NAME: &'static str;
    fn present_modification_menu(&mut self);
    fn new(track: &'ff Track) -> Option<Self>;
    fn to_string(&self) -> String;
}

fn show_codecs_menu<'ff, T: Menuable<'ff>>(entries: &mut Vec<T>, tracks: &'ff [Track]) {
    let mut v = Vec::with_capacity(entries.len()+1);
    v.push(MenuOption {value: None, label: "Add track".into()});
    v.extend(
        entries.iter()
        .enumerate()
        .map(|(i, entry)| MenuOption {value: Some(i), label: entry.to_string()})
    );
    let mut menu = Menu::new(v, MenuProps {
        title: T::MENU_NAME,
        ..MenuProps::default()
    });
    loop {
        match menu.show() {
            None => return,
            Some(None) => {
                if tracks.len() == 1 {
                    if let Some(entry) = T::new(&tracks[0]) {
                        entries.push(entry);
                    }
                } else {
                    let mut menu = Menu::new(
                        tracks.iter()
                        .map(|track| MenuOption {label: track.to_string(), value: track})
                        .collect(),
                        MenuProps {
                            title: "Select track to source from",
                            ..MenuProps::default()
                        }
                    );
                    if let Some(track) = menu.show() {
                        if let Some(entry) = T::new(track) {
                            entries.push(entry);
                        }
                    }
                }
                    
            },
            Some(Some(idx)) => {
                entries[*idx].present_modification_menu();
            },
        }
    }
}
