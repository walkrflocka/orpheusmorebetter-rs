use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use super::format::{self, Format};

static PRE_EMPHASIS_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)pre[- ]?emphasi(s(ed)?|zed)").expect("pre-emphasis regex"));

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Torrent {
    pub id: u64,
    #[serde(default)]
    pub group_id: Option<u64>,
    pub media: String,
    pub format: String,
    pub seeders: u32,
    pub snatched: u32,
    pub description: String,
    pub remaster_year: Option<u32>,
    pub remaster_title: String,
    pub remaster_record_label: String,
    pub remaster_catalogue_number: String,
    pub file_list: String,
    pub file_path: String,
    pub encoding: String,
}

impl Torrent {
    pub fn formatted_media_info(&self) -> String {
        if !self.remaster_title.is_empty() {
            let year = self
                .remaster_year
                .map(|y| y.to_string())
                .unwrap_or_default();
            format!("{{{} ~ {} {}}}", self.media, self.remaster_title, year)
        } else {
            format!("{{{}}}", self.media)
        }
    }

    pub fn allowed_transcodes(&self) -> Vec<Format> {
        if PRE_EMPHASIS_RE.is_match(&self.remaster_title) {
            Vec::new()
        } else {
            format::ALL_FORMATS.to_vec()
        }
    }

    pub fn format_key(&self) -> Option<Format> {
        format::format_from_name_encoding(&self.format, &self.encoding)
    }
}
