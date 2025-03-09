use std::{ffi::OsString, path::PathBuf};

use clap::Parser;
use console_menu::{Menu, MenuOption, MenuProps};
use cytrans::{codecs::get_capabilities, ffprobe::{ffprobe, TrackType}};

#[derive(clap::Parser)]
#[command(version, about)]
struct Args {
    input_path_or_url: OsString,
    output_directory: PathBuf,
    url_prefix: String,
}


enum MainMenuAction {
    VideoTracks,
    AudioTracks,
    Go,
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

    let mut main_menu = Menu::new(vec![
        MenuOption {
            label: "Video tracks".into(),
            value: MainMenuAction::VideoTracks,
        },
        MenuOption {
            label: "Audio tracks".into(),
            value: MainMenuAction::AudioTracks,
        },
        MenuOption {
            label: "Done, launch ffmpeg".into(),
            value: MainMenuAction::Go,
        },
    ],
    MenuProps {
        title: "Main menu",
        ..MenuProps::default()
    });
    loop {
        match main_menu.show() {
            Some(MainMenuAction::VideoTracks) => {
            },
            Some(MainMenuAction::AudioTracks) => {
            },
            Some(MainMenuAction::Go) => break,
            None => {
                println!("User exited from main menu, not running ffmpeg.");
                return;
            },
        }
    }

    // TODO: implement ffmpeg
}
