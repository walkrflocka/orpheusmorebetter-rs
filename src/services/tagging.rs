use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

use anyhow::Context;
use id3::TagLike;

const NUMERIC_TAGS: &[&str] = &[
    "tracknumber",
    "discnumber",
    "tracktotal",
    "totaltracks",
    "disctotal",
    "totaldiscs",
];

fn scrub_tag(name: &str, value: &str) -> String {
    let mut scrubbed = value.trim().trim_matches('\0').to_string();

    if NUMERIC_TAGS.contains(&name) {
        let re = regex::Regex::new(r"/(0+)?$").unwrap();
        scrubbed = re.replace(&scrubbed, "").to_string();
        scrubbed = scrubbed.trim_start_matches('/').to_string();

        if name != "tracknumber" {
            let re_zero = regex::Regex::new(r"^0+(/.*)?$").unwrap();
            if re_zero.is_match(&scrubbed) {
                return String::new();
            }
        }
    }

    scrubbed
}

fn read_flac_tags(path: &Path) -> anyhow::Result<HashMap<String, Vec<String>>> {
    let output = Command::new("metaflac")
        .args(["--export-tags-to=-"])
        .arg(path)
        .output()
        .context("Failed to run metaflac")?;

    let stdout = String::from_utf8(output.stdout)?;
    let mut tags: HashMap<String, Vec<String>> = HashMap::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=') {
            tags.entry(key.to_uppercase())
                .or_default()
                .push(value.to_string());
        }
    }
    Ok(tags)
}

fn read_mp3_tags(path: &Path) -> anyhow::Result<HashMap<String, String>> {
    let mut tags = HashMap::new();
    match id3::Tag::read_from_path(path) {
        Ok(tag) => {
            if let Some(title) = tag.title() {
                tags.insert("TITLE".to_string(), title.to_string());
            }
            if let Some(artist) = tag.artist() {
                tags.insert("ARTIST".to_string(), artist.to_string());
            }
            if let Some(album) = tag.album() {
                tags.insert("ALBUM".to_string(), album.to_string());
            }
            if let Some(track) = tag.track() {
                tags.insert("TRACKNUMBER".to_string(), track.to_string());
            }
        }
        Err(_) => {}
    }
    Ok(tags)
}

pub fn check_tags(path: &Path, check_tracknumber_format: bool) -> (bool, Option<String>) {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let tags: HashMap<String, String> = match ext.as_str() {
        "flac" => {
            let raw = match read_flac_tags(path) {
                Ok(t) => t,
                Err(e) => return (false, Some(format!("Failed to read tags: {}", e))),
            };
            raw.into_iter()
                .map(|(k, v)| (k, v.first().cloned().unwrap_or_default()))
                .collect()
        }
        "mp3" => match read_mp3_tags(path) {
            Ok(t) => t,
            Err(e) => return (false, Some(format!("Failed to read tags: {}", e))),
        },
        _ => return (false, Some(format!("Unsupported format: {}", ext))),
    };

    for required in &["ARTIST", "ALBUM", "TITLE", "TRACKNUMBER"] {
        match tags.get(*required) {
            None => {
                return (
                    false,
                    Some(format!("{:?} has no {} tag", path, required.to_lowercase())),
                );
            }
            Some(v) if v.is_empty() => {
                return (
                    false,
                    Some(format!(
                        "{:?} has an empty {} tag",
                        path,
                        required.to_lowercase()
                    )),
                );
            }
            _ => {}
        }
    }

    if check_tracknumber_format {
        if let Some(tracknumber) = tags.get("TRACKNUMBER") {
            let re = regex::Regex::new(r"^\d+(/(\d+))?$").unwrap();
            if !re.is_match(tracknumber) {
                return (
                    false,
                    Some(format!(
                        "{:?} has a malformed tracknumber tag ({:?})",
                        path, tracknumber
                    )),
                );
            }
        }
    }

    (true, None)
}

fn vorbis_to_id3_frame(key: &str) -> Option<&'static str> {
    match key {
        "ARTIST" => Some("TPE1"),
        "ALBUM" => Some("TALB"),
        "TITLE" => Some("TIT2"),
        "TRACKNUMBER" => Some("TRCK"),
        "DISCNUMBER" => Some("TPOS"),
        "DATE" => Some("TDRC"),
        "GENRE" => Some("TCON"),
        "ALBUMARTIST" | "ALBUM ARTIST" => Some("TPE2"),
        "GROUPING" | "CONTENT GROUP" => Some("TIT1"),
        _ => None,
    }
}

pub fn copy_tags(flac_file: &Path, transcode_file: &Path) -> anyhow::Result<()> {
    let flac_tags = read_flac_tags(flac_file)?;
    let ext = transcode_file
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "flac" => copy_tags_to_flac(&flac_tags, transcode_file),
        "mp3" => copy_tags_to_mp3(&flac_tags, transcode_file),
        _ => anyhow::bail!("Unsupported tag format: {}", ext),
    }
}

fn copy_tags_to_flac(tags: &HashMap<String, Vec<String>>, output: &Path) -> anyhow::Result<()> {
    for (key, values) in tags {
        for value in values {
            let scrubbed = scrub_tag(&key.to_lowercase(), value);
            if !scrubbed.is_empty() {
                let tag_str = format!("{}={}", key, scrubbed);
                Command::new("metaflac")
                    .args(["--set-tag", &tag_str])
                    .arg(output)
                    .output()
                    .context("Failed to set FLAC tag")?;
            }
        }
    }
    Ok(())
}

fn copy_tags_to_mp3(tags: &HashMap<String, Vec<String>>, output: &Path) -> anyhow::Result<()> {
    let mut id3_tag = id3::Tag::new();

    for (key, values) in tags {
        let scrubbed = scrub_tag(&key.to_lowercase(), &values.join("/"));
        if scrubbed.is_empty() {
            continue;
        }

        if let Some(frame_id) = vorbis_to_id3_frame(key) {
            if frame_id == "TRCK" || frame_id == "TPOS" {
                continue;
            }
            id3_tag.add_frame(id3::frame::Frame::with_content(
                frame_id,
                id3::frame::Content::Text(scrubbed),
            ));
        }
    }

    if let Some(tracknumber_vals) = tags.get("TRACKNUMBER") {
        let tracknumber = scrub_tag("tracknumber", &tracknumber_vals[0]);
        if !tracknumber.is_empty() {
            let total = tags
                .get("TOTALTRACKS")
                .or_else(|| tags.get("TRACKTOTAL"))
                .map(|v| scrub_tag("totaltracks", &v[0]))
                .filter(|v| !v.is_empty());

            let trck = match total {
                Some(t) => format!("{}/{}", tracknumber, t),
                None => tracknumber,
            };
            id3_tag.add_frame(id3::frame::Frame::with_content(
                "TRCK",
                id3::frame::Content::Text(trck),
            ));
        }
    }

    if let Some(discnumber_vals) = tags.get("DISCNUMBER") {
        let discnumber = scrub_tag("discnumber", &discnumber_vals[0]);
        if !discnumber.is_empty() {
            let total = tags
                .get("TOTALDISCS")
                .or_else(|| tags.get("DISCTOTAL"))
                .map(|v| scrub_tag("totaldiscs", &v[0]))
                .filter(|v| !v.is_empty());

            let tpos = match total {
                Some(t) => format!("{}/{}", discnumber, t),
                None => discnumber,
            };
            id3_tag.add_frame(id3::frame::Frame::with_content(
                "TPOS",
                id3::frame::Content::Text(tpos),
            ));
        }
    }

    if let Some(comment_vals) = tags.get("COMMENT") {
        let comment_text = scrub_tag("comment", &comment_vals[0]);
        if !comment_text.is_empty() {
            id3_tag.add_frame(id3::frame::Frame::with_content(
                "COMM",
                id3::frame::Content::Comment(id3::frame::Comment {
                    lang: "eng".to_string(),
                    description: String::new(),
                    text: comment_text,
                }),
            ));
        }
    }

    if let Some(orig_date_vals) = tags.get("ORIGINALDATE") {
        let orig_date = scrub_tag("originaldate", &orig_date_vals[0]);
        if !orig_date.is_empty() {
            id3_tag.add_frame(id3::frame::Frame::with_content(
                "TDOR",
                id3::frame::Content::Text(orig_date),
            ));
        }
    }

    id3_tag
        .write_to_path(output, id3::Version::Id3v24)
        .context("Failed to write ID3 tags")?;

    Ok(())
}
