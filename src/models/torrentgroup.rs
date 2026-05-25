use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use super::artist::Artist;
use super::format::Format;
use super::torrent::Torrent;

static INVALID_CHARS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"[\?<>\\*\|":/]"#).expect("invalid chars regex"));

#[derive(Clone, Debug)]
pub struct TorrentGroup {
    pub id: u64,
    pub name: String,
    pub year: u32,
    pub torrents: Vec<Torrent>,
    pub composers: Vec<Artist>,
    pub dj: Vec<Artist>,
    pub artists: Vec<Artist>,
    pub with_artists: Vec<Artist>,
    pub conductor: Vec<Artist>,
    pub remixed_by: Vec<Artist>,
    pub producer: Vec<Artist>,
}

#[derive(Deserialize)]
pub struct TorrentGroupApiResponse {
    pub group: GroupData,
    pub torrents: Vec<Torrent>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GroupData {
    pub group_id: u64,
    pub group_name: String,
    pub group_year: u32,
    pub music_info: MusicInfo,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MusicInfo {
    #[serde(default)]
    pub composers: Vec<Artist>,
    #[serde(default)]
    pub dj: Vec<Artist>,
    #[serde(default)]
    pub artists: Vec<Artist>,
    #[serde(default, rename = "with")]
    pub with_artists: Vec<Artist>,
    #[serde(default)]
    pub conductor: Vec<Artist>,
    #[serde(default)]
    pub remixed_by: Vec<Artist>,
    #[serde(default)]
    pub producer: Vec<Artist>,
}

impl TorrentGroup {
    pub fn from_response(resp: TorrentGroupApiResponse) -> Self {
        let mut torrents = resp.torrents;
        for t in &mut torrents {
            t.group_id = Some(resp.group.group_id);
        }

        TorrentGroup {
            id: resp.group.group_id,
            name: resp.group.group_name,
            year: resp.group.group_year,
            torrents,
            composers: resp.group.music_info.composers,
            dj: resp.group.music_info.dj,
            artists: resp.group.music_info.artists,
            with_artists: resp.group.music_info.with_artists,
            conductor: resp.group.music_info.conductor,
            remixed_by: resp.group.music_info.remixed_by,
            producer: resp.group.music_info.producer,
        }
    }

    pub fn formatted_artist_string(&self) -> String {
        let primary: &[Artist] = if !self.composers.is_empty() {
            &self.composers
        } else if !self.dj.is_empty() {
            &self.dj
        } else if !self.artists.is_empty() {
            &self.artists
        } else {
            return "Unknown".to_string();
        };

        match primary.len() {
            0 => "Unknown".to_string(),
            1 => primary[0].name.clone(),
            2 => format!("{} & {}", primary[0].name, primary[1].name),
            _ => {
                let mut names: Vec<String> = primary.iter().map(|a| a.name.clone()).collect();
                let last = names.pop().unwrap();
                let result = format!("{}, & {}", names.join(", "), last);
                if result.len() > 50 {
                    "Various Artists".to_string()
                } else {
                    result
                }
            }
        }
    }

    pub fn get_transcode_dirname(&self, source: &Torrent, target_format: &Format) -> String {
        let name_truncated: String = self.name.chars().take(100).collect();
        let dirname = format!(
            "{} - {} - {} {} [{}]",
            self.formatted_artist_string(),
            self.year,
            name_truncated,
            source.formatted_media_info(),
            target_format.long_name
        );
        INVALID_CHARS_RE.replace_all(&dirname, "_").to_string()
    }
}
