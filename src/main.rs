mod app_config;
mod sessions;
mod whatapi;

use app_config::{AppConfig, Format, LosslessMediaSources};

use crate::whatapi::WhatAPI;

// single thread async
#[tokio::main(flavor = "current_thread")]
async fn main() {
    let _conf = AppConfig {
        username: "foo",
        password: "bar",
        data_dir: "~/.omb/data_dir",
        output_dir: "~/.omb/output_dir",
        torrent_dir: "~/.omb/torrent_dir",
        formats: vec![Format::Mp3320, Format::FLAC],
        media: vec![LosslessMediaSources::CD],
    };

    let api = WhatAPI::new(
        "---".to_string(),
        "---".to_string(),
        Some("https://orpheus.network/".to_string()),
        None,
    )
    .await
    .expect("Init failed");

    println!("{:?}", api);
    println!("{:?}", api.api_session);

    return ();
}
