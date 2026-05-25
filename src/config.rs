use std::collections::HashSet;
use std::path::{Path, PathBuf};

use anyhow::Context;
use ini::Ini;

use crate::models::format::{self, Format};

pub const LOSSLESS_MEDIA: &[&str] = &[
    "cd",
    "dvd",
    "vinyl",
    "soundboard",
    "sacd",
    "dat",
    "web",
    "blu-ray",
];

pub const MEDIA_SEARCH_MAP: &[(&str, &str)] = &[
    ("cd", "CD"),
    ("dvd", "DVD"),
    ("vinyl", "Vinyl"),
    ("soundboard", "Soundboard"),
    ("sacd", "SACD"),
    ("dat", "DAT"),
    ("web", "WEB"),
    ("blu-ray", "Blu-ray"),
];

pub struct AppConfig {
    pub username: String,
    pub password: String,
    pub data_dirs: Vec<PathBuf>,
    pub output_dir: PathBuf,
    pub torrent_dir: PathBuf,
    pub formats: Vec<Format>,
    pub media: HashSet<String>,
    pub tracker: String,
    pub api_endpoint: String,
    pub mode: String,
    pub source: Option<String>,
    pub do_24_bit: u8,
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = std::env::var_os("HOME") {
            return format!("{}/{}", home.to_string_lossy(), rest);
        }
    }
    path.to_string()
}

fn parse_formats(s: &str) -> Vec<Format> {
    let mut formats = Vec::new();
    for part in s.split(',') {
        let cleaned = part.trim().to_uppercase();
        if cleaned.contains("FLAC") {
            formats.push(format::FLAC);
        } else if cleaned.contains("320") {
            formats.push(format::MP3_320);
        } else if cleaned.contains("V0") {
            formats.push(format::MP3_V0);
        }
    }
    formats
}

pub fn load_config(path: &str) -> anyhow::Result<AppConfig> {
    let expanded = expand_tilde(path);
    let path = Path::new(&expanded);

    if !path.exists() {
        create_default_config(path)?;
        anyhow::bail!(
            "Created default config at {}. Please edit it and re-run.",
            path.display()
        );
    }

    let ini = Ini::load_from_file(path).context("Failed to load config file")?;
    let section = ini
        .section(Some("orpheus"))
        .context("Missing [orpheus] section in config")?;

    let username = section
        .get("username")
        .context("Missing username")?
        .to_string();
    let password = section
        .get("password")
        .context("Missing password")?
        .to_string();

    let data_dir_str = section.get("data_dir").context("Missing data_dir")?;
    let data_dirs: Vec<PathBuf> = data_dir_str
        .split(';')
        .map(|d| PathBuf::from(expand_tilde(d.trim())))
        .collect();

    let output_dir = section
        .get("output_dir")
        .filter(|s| !s.is_empty())
        .map(|s| PathBuf::from(expand_tilde(s)))
        .unwrap_or_else(|| data_dirs[0].clone());

    let torrent_dir = PathBuf::from(expand_tilde(
        section.get("torrent_dir").context("Missing torrent_dir")?,
    ));

    let formats = parse_formats(section.get("formats").unwrap_or("flac, v0, 320"));

    let media: HashSet<String> = match section.get("media") {
        Some(s) if !s.is_empty() => s.split(',').map(|m| m.trim().to_lowercase()).collect(),
        _ => LOSSLESS_MEDIA.iter().map(|s| s.to_string()).collect(),
    };

    let mut tracker = section
        .get("tracker")
        .unwrap_or("https://home.opsfet.ch/")
        .to_string();
    if !tracker.ends_with('/') {
        tracker.push('/');
    }

    let mut api_endpoint = section
        .get("api")
        .unwrap_or("https://orpheus.network/")
        .to_string();
    if !api_endpoint.ends_with('/') {
        api_endpoint.push('/');
    }

    let mode = section.get("mode").unwrap_or("both").to_string();

    let source = section
        .get("source")
        .filter(|s| !s.is_empty())
        .map(String::from);

    let do_24_bit: u8 = section
        .get("24bit_behaviour")
        .unwrap_or("0")
        .parse()
        .unwrap_or(0);

    Ok(AppConfig {
        username,
        password,
        data_dirs,
        output_dir,
        torrent_dir,
        formats,
        media,
        tracker,
        api_endpoint,
        mode,
        source,
        do_24_bit,
    })
}

fn create_default_config(path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut ini = Ini::new();
    let mut section = ini.with_section(Some("orpheus"));
    section.set("username", "");
    section.set("password", "");
    section.set("data_dir", "");
    section.set("output_dir", "");
    section.set("torrent_dir", "");
    section.set("formats", "flac, v0, 320");
    section.set("media", LOSSLESS_MEDIA.join(", "));
    section.set("24bit_behaviour", "0");
    section.set("tracker", "https://home.opsfet.ch/");
    section.set("mode", "both");
    section.set("api", "https://orpheus.network/");
    section.set("source", "OPS");

    ini.write_to_file(path)?;
    Ok(())
}

pub fn output_dir_for_format(config: &AppConfig, format: &Format) -> PathBuf {
    let key = match format.long_name {
        "MP3 V0" => "output_dir_v0",
        "MP3 320" => "output_dir_320",
        "FLAC" => "output_dir_flac",
        _ => return config.output_dir.clone(),
    };
    // Re-read from config file for per-format override
    // For now, just use the default
    config.output_dir.clone()
}

pub fn torrent_dir_for_format(config: &AppConfig, format: &Format) -> PathBuf {
    let key = match format.long_name {
        "MP3 V0" => "torrent_dir_v0",
        "MP3 320" => "torrent_dir_320",
        "FLAC" => "torrent_dir_flac",
        _ => return config.torrent_dir.clone(),
    };
    config.torrent_dir.clone()
}
