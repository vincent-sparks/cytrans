use std::{ffi::OsStr, io::Read as _};

use isahc::http::{uri::PathAndQuery, Uri};
use serde::de::DeserializeSeed;

#[derive(serde::Deserialize,Debug)]
struct JellyfinResponse {
    #[serde(rename="SeriesName")]
    series_name: Option<String>,
    #[serde(rename="Name")]
    title: String,
    #[serde(rename="ParentIndexNumber")]
    season: Option<u32>,
    #[serde(rename="IndexNumber")]
    episode: Option<u32>,
}

// i am flabbergasted that i have to do this manually.
struct JellyfinResponseSeed;

impl<'de> serde::de::DeserializeSeed<'de> for JellyfinResponseSeed {
    type Value=JellyfinResponse;

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de> {
            deserializer.deserialize_map(self)
    }
}

impl<'de> serde::de::Visitor<'de> for JellyfinResponseSeed {
    type Value=JellyfinResponse;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("jellyfin response struct")
    }
    fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
    where
        A: serde::de::MapAccess<'de>,
    {
        let mut res = None;
        while let Some(k) = map.next_key::<String>()? {
            if k == "Items" {
                res = Some(map.next_value::<[JellyfinResponse;1]>().map(|[x]| x)?);
            } else {
                map.next_value::<serde::de::IgnoredAny>();
            }
        }
        use serde::de::Error;
        res.ok_or_else(|| A::Error::missing_field("Items"))
    }

}

pub fn get_jellyfin_title(input: &OsStr) -> Option<String> {
    let input = input.to_str()?;
    let mut parts = Uri::try_from(input).ok()?.into_parts();

    let p = parts.path_and_query.as_ref()?;

    let item_id = p.path().strip_prefix("/Items/").and_then(|x| x.strip_suffix("/Download"));

    if item_id.is_none() {
        println!("not a jellyfin url");
    }
    
    let item_id = item_id?;

    let Some(query) = p.query() else {
        println!("no api key in url");
        return None;
    };

    let new_path_and_query = format!("/Items?ids={}&{}", item_id, query);

    let new_path_and_query = PathAndQuery::try_from(new_path_and_query).expect("failed generating new path and query");

    parts.path_and_query = Some(new_path_and_query);

    let new_uri = Uri::try_from(parts).expect("failed generating new url");

    println!("sending request to {}", new_uri);

    let mut response = match isahc::get(new_uri) {
        Ok(r) => r,
        Err(e) => {
            println!("failed querying filename from jellyfin: {}", e);
            return None;
        }
    };

    let mut deserializer = serde_json::Deserializer::from_reader(response.body_mut());

    let response = match JellyfinResponseSeed.deserialize(&mut deserializer) {
        Ok(r) => r,
        Err(e) => {
            println!("error deserializing: {}", e);
            return None;
        }
    };

    println!("got it! {:?}", response);
    if let (Some(series_name), Some(season), Some(episode)) = (&response.series_name, &response.season, &response.episode) {
        Some(format!("{} S{}E{:02}: {}", series_name, season, episode, response.title))
    } else {
        Some(response.title)
    }
}
