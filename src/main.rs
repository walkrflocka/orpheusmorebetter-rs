mod app_config;
mod sessions;
mod whatapi;

use app_config::{AppConfig, Format, LosslessMediaSources};

use crate::whatapi::WhatAPI;

fn main() {
    let _conf = AppConfig {
        username: "foo",
        password: "bar",
        data_dir: "~/.omb/data_dir",
        output_dir: "~/.omb/output_dir",
        torrent_dir: "~/.omb/torrent_dir",
        formats: vec![Format::Mp3320, Format::FLAC],
        media: vec![LosslessMediaSources::CD],
    };

    let _api = WhatAPI::new(
        "PUT HERE".to_string(),
        "EDIT".to_string(),
        Some("https://orpheus.network/".to_string()),
        None,
    );

    return ();
}
