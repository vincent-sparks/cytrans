use cytrans::{ffprobe::{Track, FFprobeResult}, options::{TranscodeRequest, TranscodeArgs}};
use serde::de::{Visitor, Deserializer, DeserializeSeed, MapAccess, SeqAccess};
use serde::Deserialize;
use std::marker::PhantomData;
use std::ffi::OsString;
use std::str::FromStr;
use std::sync::Arc;
use crate::ser::*;
use crate::state::State;

macro_rules! maybe_unwrap {
    (o, $i:ident) => {};
    (r, $i:ident) => {let $i = $i.ok_or_else(|| serde::de::Error::missing_field(stringify!($i)))?;}
}

macro_rules! type_name {
    ($a: ident) => {$a};
    ($a:ident<$($ignore:tt),*>) => {$a};
}

macro_rules! next_value_seed {
    ($map: ident) => {$map.next_value()?};
    ($map: ident, $seed: expr) => {$map.next_value_seed($seed)?};
}

macro_rules! implement_de_seed {
    ($struct: ident, {$($params:tt)*}, {$($where:tt)*}, $value:ty, $method: ident) => {
        impl<'de, $($params)*> DeserializeSeed<'de> for $struct<$($params)*> where $($where)* {
            type Value = $value;
            fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
                de.$method(self)
            }
        }
    }
}

macro_rules! maybe_some {
    (r, $expr:expr) => {Some($expr)};
    (o, $expr:expr) => {$expr};
}

macro_rules! default_result {
    ($struct: ident { $($i:ident),*}) => {return Ok($struct{$($i),*})};
    ($struct: ident { $($i:ident),*} {$($cust:tt)*}) => {$($cust)*};
}

macro_rules! generate_visitor {
    ($struct: ident, $result: ty, $visitor: ident, $enum: ident | {$($params:tt)*} {$($where:tt)*} | $self:ident | $($o:tt $i: ident $(= $v:expr)?),* $(,)? $(;{$($finalize:tt)*})? ) => {
        #[derive(Deserialize)]
        #[allow(non_camel_case_types)]
        enum $enum {$($i),*}

        impl<'de, $($params)*> Visitor<'de> for $visitor<$($params)*> where $($where)* {
            type Value = $result;
            fn visit_map<M: MapAccess<'de>>($self, mut map:M) -> Result<Self::Value, M::Error> {
                $(let mut $i = None;)*
                while let Some(key) = map.next_key::<$enum>()? {
                    match key {
                    $(
                        $enum::$i => {
                            if $i.is_some() {
                                return Err(serde::de::Error::duplicate_field(stringify!($i)));
                            }
                            $i = maybe_some!($o, next_value_seed!(map $(, $v)?));
                        }
                    )*
                    }
                }
                $(maybe_unwrap!($o, $i);)*
                default_result!($struct {$($i),*} $({$($finalize)*})?);
            }
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(concat!("a valid ", stringify!($struct), " structure"))
            }
        }
        impl<'de, $($params)*> DeserializeSeed<'de> for $visitor<$($params)*> where $($where)* {
            type Value = $result;
            fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
                de.deserialize_struct(stringify!($struct), &[$(stringify!($i)),*], self)
            }
        }
    }
}

pub struct TranscodeArgsDeserializer<'ff> {
    pub tracks: &'ff Vec<Track>,
    pub duration: f32,
}

generate_visitor!(TranscodeArgs, TranscodeArgs<'ff>, TranscodeArgsDeserializer, TranscodeArgsFields |
                  {'ff} {} |
                  self |
                  r video_tracks = VecSeed(RequestSeed(self.tracks, PhantomData)),
                  r audio_tracks = VecSeed(RequestSeed(self.tracks, PhantomData)),
                  r subtitle_tracks = VecSeed(TrackSeed(self.tracks)),
                  r title,
                  r extra_ffmpeg_args = VecSeed(OsStringSeed);

                  {return Ok(TranscodeArgs {video_tracks, audio_tracks, subtitle_tracks, title, extra_ffmpeg_args, duration: self.duration} )}
                  );

#[derive(Clone, Copy)]
struct TrackSeed<'a>(&'a Vec<Track>);

impl<'de, 'ff> DeserializeSeed<'de> for TrackSeed<'ff> {
    type Value = &'ff Track;
    fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
        de.deserialize_u16(self)
    }
}

impl<'de, 'ff> Visitor<'de> for TrackSeed<'ff> {
    type Value = &'ff Track;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a track index")
    }
    fn visit_u16<E: serde::de::Error>(self, track_idx: u16) -> Result<Self::Value, E> {
        for track in self.0.iter() {
            if track.index == track_idx {
                return Ok(track);
            }
        }
        return Err(E::custom(format_args!("No such track: {}", track_idx)));
    }

    // extremely cool of serde_json to emit visit_u64() regardless of what the visitor asked for
    fn visit_u64<E: serde::de::Error>(self, n: u64) -> Result<Self::Value, E> {
        self.visit_u16(n.try_into().map_err(E::custom)?) 
    }
}

#[derive(Clone, Copy)]
struct RequestSeed<'a, T>(&'a Vec<Track>, PhantomData<T>);

generate_visitor!(TranscodeRequest, TranscodeRequest<'ff, T>, RequestSeed, RequestFields |
                  {'ff, T} {T: serde::de::Deserialize<'de>} |
                  self |
                  r track = TrackSeed(self.0),
                  o bitrate,
                  r codec,
                  r encoder,
                  r extra_ffmpeg_args = VecSeed(OsStringSeed)
                  );

struct VecSeed<T>(T);

impl<'de, 'ff, T> DeserializeSeed<'de> for VecSeed<T> where T: Visitor<'de> + DeserializeSeed<'de> + Clone + Copy {
    type Value = Vec<<T as DeserializeSeed<'de>>::Value>;
    fn deserialize<D: Deserializer<'de>>(self, de: D) -> Result<Self::Value, D::Error> {
        de.deserialize_seq(self)
    }
}

impl<'de, 'ff, T> Visitor<'de> for VecSeed<T> where T: DeserializeSeed<'de> + Visitor<'de> + Clone + Copy {
    type Value = Vec<<T as DeserializeSeed<'de>>::Value>;
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("a list, where each element is ")?;
        self.0.expecting(f)
    }
    fn visit_seq<S: SeqAccess<'de>>(self, mut seq: S) -> Result<Self::Value, S::Error> {
        let mut v = match seq.size_hint() {
            Some(x) => Vec::with_capacity(x),
            None => Vec::new(),
        };
        while let Some(val) = seq.next_element_seed(self.0)? {
            v.push(val);
        }
        Ok(v)
    }
}

#[derive(Clone,Copy)]
struct OsStringSeed;
implement_de_seed!(OsStringSeed, {}, {}, OsString, deserialize_str);

impl<'de> Visitor<'de> for OsStringSeed {
    type Value = OsString;
    fn visit_str<E: serde::de::Error>(self, s: &str) -> Result<OsString, E> {
        OsString::from_str(s).map_err(|x| match x {})
    }
    fn visit_enum<A:serde::de::EnumAccess<'de>>(self, a:A) -> Result<OsString, A::Error> {
        OsString::deserialize(serde::de::value::EnumAccessDeserializer::new(a))
    }
    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("an OsString")
    }
}

/*
pub struct TranscodeJobDeserializer{
    pub file: std::path::PathBuf,
    pub ffprobe: Arc<FFprobeResult>
};

generate_visitor!(TranscodeJob, TranscodeJob, TranscodeJobDeserializer, TranscodeJobFields | {} {} | self |
                  r args = TranscodeArgsDeserializer{tracks: &self.1.tracks, duration: self.1.duration},
                  r slug
                  ; {
                      let (command, ctvideo) = cytrans::transcode::build_ffmpeg_command(&self.0, args, false, false, "/balls".as_ref(), "https://nowhere.com/");
                        return Ok(TranscodeJob {command: command.into(), slug, duration: self.1.duration});
                  }
                  );

#[derive(serde::Deserialize)]
struct PathArg{path:String}

use axum::extract::rejection::QueryRejection;
use crate::state::{BadPath, FFprobeError};

use crate::rejection_enum;
rejection_enum!(TranscodeArgsRejection, {QueryRejection, BadPath, FFprobeError});

#[async_trait::async_trait]
impl axum::extract::FromRequestParts<Arc<State>> for TranscodeJobDeserializer {
    type Rejection = TranscodeArgsRejection;
    async fn from_request_parts(parts: &mut http::request::Parts, state: &Arc<State>) -> Result<Self, Self::Rejection> {
        use TranscodeArgsRejection::*;
        let a = axum::extract::Query::<PathArg>::from_request_parts(parts, &()).await.map_err(QueryRejection)?;
        let path = state.sanitize_path(&a.path).map_err(BadPath)?;
        let ffprobe = state.ffprobe(&path).map_err(FFprobeError)?;
        
        Ok(TranscodeJobDeserializer(path, ffprobe))
    }
}


impl TranscodeJobDeserializer {

}
*/
