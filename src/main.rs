mod ajax;
mod app_config;
mod extensions;
mod whatapi;

use app_config::{AppConfig, Format, LosslessMediaSources};

fn main() {
    let conf = AppConfig {
        username: "foo",
        password: "bar",
        data_dir: "~/.omb/data_dir",
        output_dir: "~/.omb/output_dir",
        torrent_dir: "~/.omb/torrent_dir",
        formats: vec![Format::Mp3320, Format::FLAC],
        media: vec![LosslessMediaSources::CD],
    };

    println!("{:?}", conf)
}
