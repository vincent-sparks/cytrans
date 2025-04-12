#![feature(never_type)]
#![feature(iter_intersperse)]
use std::{ffi::OsString, fmt::Display, os::unix::process::CommandExt as _, path::PathBuf, str::FromStr};

use clap::Parser;
use console_menu::{Menu, MenuOption, MenuProps};
use cytrans::{codecs::get_capabilities, ffprobe::{ffprobe, Track, TrackType}, options::{AudioCodec, TrackOptions, TranscodeArgs, VideoCodec}, transcode::build_ffmpeg_command};

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

    if !args.output_directory.is_dir() {
        std::fs::create_dir(&args.output_directory).expect("Error creating output directory");
    }

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

    let input_audio_tracks = ffprobe_result.tracks.iter().filter(|x| x.kind == TrackType::Audio).collect::<Vec<_>>();

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

    let mut title = ffprobe_result.title.clone().unwrap_or_else(|| {
        let filename = args.input_path_or_url.to_string_lossy();
        let mut out = &filename[filename.rfind('/').map(|x|x+1).unwrap_or(0)..];
        if filename.starts_with("http") {
            if let Some(pos) = out.find('?') {
                out = &out[..pos];
            }
        }
        out.to_string()
    });
    let extra_ffmpeg_args = Vec::new(); // TODO add a way to specify extra ffmpeg args
    

    let mut main_menu = Menu::new(
        MainMenuAction::into_menu(),
        MenuProps {
            title: "Main menu",
            ..MenuProps::default()
        }
    );

    let mut line_editor = rustyline::Editor::<(),_>::new().expect("error creating rustyline editor");

    loop {
        match main_menu.show() {
            Some(MainMenuAction::VideoTracks) => {
                show_tracks_menu(&mut video_tracks, video_track.as_slice(), &mut line_editor);
            },
            Some(MainMenuAction::AudioTracks) => {
                show_tracks_menu(&mut audio_tracks, &input_audio_tracks, &mut line_editor);
            },
            Some(MainMenuAction::Title) => {
                if let Ok(new_title) = line_editor.readline_with_initial("Title: ", (&title,"")) {
                    title = new_title;
                }
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

    let (mut command, metadata_manifest, _did_demux) = build_ffmpeg_command(&args.input_path_or_url, transcode_args, &args.output_directory);

    serde_json::to_writer(
        std::fs::File::create(args.output_directory.join("manifest.json")).expect("error creating the manifest JSON file"),
        &metadata_manifest.to_cytube(&args.url_prefix),
    ).expect("Error writing the manifest JSON file");

    let error = command.exec();

    panic!("Error invoking ffmpeg: {}", error);

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
    const ENCODER_LIST_NAME: &'static str;
    fn label_for(options: &TrackOptions<'ff, Self>) -> String;
    fn get_encoders() -> &'static [(Self, Vec<String>)];
}

#[derive(strum::EnumMessage,strum::EnumIter)]
enum ModifyEntryMenu {
    #[strum(message="Change source track")]
    ChangeTrack,
    #[strum(message="Change codec")]
    ChangeCodec,
    #[strum(message="Change ffmpeg args")]
    ChangeFfmpegArgs,
    #[strum(message="Remove this output track")]
    DeleteTrack,
    #[strum(message="Done, go back")]
    Done,
}

fn ask_if_sure(message: &str) -> bool {
    Menu::new(
        vec![
        MenuOption {label: "No".into(), value: false},
        MenuOption {label: "Yes".into(), value: true},
        ],
        MenuProps {
            title: message,
            ..MenuProps::default()
        }
    )
        .show()
        .copied()
        .unwrap_or(false)
}

fn choose_track<'ff>(title: &str, input_tracks: &[&'ff Track]) -> Option<&'ff Track> {
    let mut menu = Menu::new(
        input_tracks.iter()
        .map(|track| MenuOption {label: track.to_string(), value: *track})
        .collect(),
        MenuProps {
            title: "Select track to source from",
            ..MenuProps::default()
        }
    );
    menu.show().map(|v|&**v)
}

fn show_tracks_menu<'ff, T: Menuable<'ff> + Copy + Display + FromStr + Into<&'static str> + 'static>(output_tracks: &mut Vec<TrackOptions<'ff, T>>, input_tracks: &[&'ff Track], editor: &mut rustyline::Editor<(), rustyline::history::DefaultHistory>) {
    if input_tracks.is_empty() {
        Menu::new(vec![MenuOption {label: "Back".into(), value: ()}], MenuProps {title: "-- no tracks available --", ..MenuProps::default()}).show();
        return;
    }

    loop {
        // must rebuild this menu every time through the loop in case the list of tracks changes.
        let mut v = Vec::with_capacity(output_tracks.len()+1);
        v.push(MenuOption {value: None, label: "Add track".into()});
        v.extend(
            output_tracks.iter()
            .enumerate()
            .map(|(i, entry)| MenuOption {value: Some(i), label: T::label_for(entry)})
        );
        let mut menu = Menu::new(v, MenuProps {
            title: T::MENU_NAME,
            ..MenuProps::default()
        });

        match menu.show() {
            None => return,
            Some(None) => {
                let chosen_track = if input_tracks.len() == 1 {
                    Some(input_tracks[0])
                } else {
                    choose_track("Select an input track to source from", input_tracks)
                };

                if let Some(track) = chosen_track {

                    let origin_codec = if output_tracks.iter().any(|stream| stream.track.index == track.index && stream.encoder=="copy") {
                        None
                    } else {
                        Some(track.codec.as_str())
                    };

                    if let Some((codec, encoder)) = choose_encoder(T::ENCODER_LIST_NAME, T::get_encoders(), origin_codec) {
                        output_tracks.push(TrackOptions {
                            track, codec,
                            encoder: encoder.into(),
                            bitrate: None,
                            extra_ffmpeg_args: vec![],
                        });
                    }
                }
                    
            },
            Some(Some(idx)) => {
                let mut modify_entry_menu = Menu::new(ModifyEntryMenu::into_menu(), MenuProps {
                    title: "Modify Track",
                    ..MenuProps::default()
                });
                if let Some(option) = modify_entry_menu.show() {
                    match option {
                        ModifyEntryMenu::ChangeTrack => {
                            if let Some(new_track) = choose_track("Choose a new track", input_tracks) {
                                output_tracks[*idx].track = new_track;
                            }
                        },
                        ModifyEntryMenu::ChangeCodec => {
                            let current_track_index = output_tracks[*idx].track.index;
                            let any_copy_already = output_tracks.iter().enumerate().any(|(i, stream)| i != *idx && stream.track.index == current_track_index && stream.encoder=="copy");
                            let origin_codec = if any_copy_already {None} else {Some(output_tracks[*idx].track.codec.as_str())};
                            if let Some((codec, encoder)) = choose_encoder(T::ENCODER_LIST_NAME, T::get_encoders(), origin_codec) {
                                output_tracks[*idx].codec = codec;
                                output_tracks[*idx].encoder = encoder.into();
                            }
                        },
                        ModifyEntryMenu::ChangeFfmpegArgs => {
                            modify_ffmpeg_args_menu(&mut output_tracks[*idx].extra_ffmpeg_args, editor);
                        },
                        ModifyEntryMenu::DeleteTrack => {
                            if ask_if_sure("Really delete?") {
                                output_tracks.remove(*idx);
                            }
                        },
                        ModifyEntryMenu::Done => {},
                    }
                }
            },
        }
    }
}



fn modify_ffmpeg_args_menu(extra_ffmpeg_args: &mut Vec<OsString>, line_editor: &mut rustyline::Editor<(), rustyline::history::DefaultHistory>) {
    println!("TODO: this interface is super basic and doesn't support spaces in arguments at all!");
    println!("TODO: I don't have time to implement a better one right now, but I really need to!");
    let s = extra_ffmpeg_args.iter().map(|x|x.to_string_lossy()).intersperse(std::borrow::Cow::Borrowed(" ")).collect::<String>();
    if let Ok(new_args) = line_editor.readline_with_initial("ffmpeg args:", (&s,"")) {
        *extra_ffmpeg_args = new_args.split(' ').map(Into::into).collect();
    }

}

impl<'ff> Menuable<'ff> for VideoCodec {
    const TRACK_TYPE: TrackType = TrackType::Video;

    const MENU_NAME: &'static str = "Video streams";

    const ENCODER_LIST_NAME: &'static str = "Select video encoder";

    fn label_for(options: &TrackOptions<'ff, Self>) -> String {
        format!("{} ({})", options.codec, options.encoder)
    }

    fn get_encoders() -> &'static [(Self, Vec<String>)] {
        cytrans::codecs::get_video_encoders()
    }
}

impl<'ff> Menuable<'ff> for AudioCodec {
    const TRACK_TYPE: TrackType = TrackType::Audio;

    const MENU_NAME: &'static str = "Audio streams";

    const ENCODER_LIST_NAME: &'static str = "Select audio encoder";

    fn label_for(options: &TrackOptions<'ff, Self>) -> String {
        format!("#{} ({}) -> {} ({})", options.track.index, options.track.language.as_ref().map(|x| x.as_str()).unwrap_or("unknown"), options.codec, options.encoder)
    }

    fn get_encoders() -> &'static [(Self, Vec<String>)] {
        cytrans::codecs::get_audio_encoders()
    }
}
