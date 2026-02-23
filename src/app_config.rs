use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub enum Format {
    FLAC,
    Mp3V0,
    Mp3320,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum LosslessMediaSources {
    CD,
    DVD,
    Vinyl,
    Soundboard,
    SACT,
    DAT,
    Web,
    BluRay,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct AppConfig<'a> {
    pub username: &'a str,
    pub password: &'a str,
    pub data_dir: &'a str,
    pub output_dir: &'a str,
    pub torrent_dir: &'a str,
    pub formats: Vec<Format>,
    pub media: Vec<LosslessMediaSources>,
}
