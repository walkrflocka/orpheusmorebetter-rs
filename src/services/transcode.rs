use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::Context;
use regex::Regex;
use walkdir::WalkDir;

use crate::models::format::Format;
use crate::models::torrent::Torrent;
use crate::models::torrentgroup::TorrentGroup;
use crate::services::tagging;

pub fn find_files(dir: &Path, extension: &str) -> anyhow::Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            if let Some(ext) = entry.path().extension() {
                if ext.to_string_lossy().to_lowercase() == extension {
                    let name = entry.file_name().to_string_lossy();
                    if !name.starts_with('.') {
                        files.push(entry.into_path());
                    }
                }
            }
        }
    }
    Ok(files)
}

pub fn get_flac_properties(path: &Path) -> anyhow::Result<(u32, u8, u8)> {
    let output = Command::new("metaflac")
        .args(["--show-sample-rate", "--show-bps", "--show-channels"])
        .arg(path)
        .output()
        .context("Failed to run metaflac")?;

    let stdout = String::from_utf8(output.stdout)?;
    let mut lines = stdout.lines();
    let sample_rate: u32 = lines
        .next()
        .context("missing sample rate")?
        .trim()
        .parse()
        .context("invalid sample rate")?;
    let bit_depth: u8 = lines
        .next()
        .context("missing bit depth")?
        .trim()
        .parse()
        .context("invalid bit depth")?;
    let channels: u8 = lines
        .next()
        .context("missing channels")?
        .trim()
        .parse()
        .context("invalid channels")?;

    Ok((sample_rate, bit_depth, channels))
}

pub fn is_24bit(flac_dir: &Path) -> anyhow::Result<bool> {
    for file in find_files(flac_dir, "flac")? {
        let (_, bit_depth, _) = get_flac_properties(&file)?;
        if bit_depth > 16 {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn is_multichannel(flac_dir: &Path) -> anyhow::Result<bool> {
    for file in find_files(flac_dir, "flac")? {
        let (_, _, channels) = get_flac_properties(&file)?;
        if channels > 2 {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn needs_resampling(flac_dir: &Path) -> anyhow::Result<bool> {
    is_24bit(flac_dir)
}

pub fn resample_rate(flac_dir: &Path) -> anyhow::Result<Option<u32>> {
    let flac_files = find_files(flac_dir, "flac")?;
    if flac_files.is_empty() {
        return Ok(None);
    }

    let mut max_rate = 0u32;
    for file in &flac_files {
        let (rate, _, _) = get_flac_properties(file)?;
        max_rate = max_rate.max(rate);
    }

    if max_rate % 44100 == 0 {
        Ok(Some(44100))
    } else if max_rate % 48000 == 0 {
        Ok(Some(48000))
    } else {
        Ok(None)
    }
}

fn build_pipeline_commands(
    format: &Format,
    resample: bool,
    sample_rate: Option<u32>,
    input: &Path,
    output: &Path,
) -> Vec<Vec<String>> {
    let input_str = input.to_string_lossy().to_string();
    let output_str = output.to_string_lossy().to_string();

    if format.name == "FLAC" && resample {
        return vec![vec![
            "sox".into(),
            input_str,
            "-G".into(),
            "-b".into(),
            "16".into(),
            output_str,
            "rate".into(),
            "-v".into(),
            "-L".into(),
            sample_rate.unwrap().to_string(),
            "dither".into(),
        ]];
    }

    let mut cmds = Vec::new();

    if resample {
        cmds.push(vec![
            "sox".into(),
            input_str,
            "-G".into(),
            "-b".into(),
            "16".into(),
            "-t".into(),
            "wav".into(),
            "-".into(),
            "rate".into(),
            "-v".into(),
            "-L".into(),
            sample_rate.unwrap().to_string(),
            "dither".into(),
        ]);
    } else {
        cmds.push(vec!["flac".into(), "-dcs".into(), "--".into(), input_str]);
    }

    match format.encoder.enc {
        "lame" => {
            let mut cmd = vec!["lame".into(), "-S".into()];
            for opt in format.encoder.opts.split_whitespace() {
                cmd.push(opt.to_string());
            }
            cmd.push("-".into());
            cmd.push(output_str);
            cmds.push(cmd);
        }
        "flac" => {
            let mut cmd = vec!["flac".into()];
            for opt in format.encoder.opts.split_whitespace() {
                cmd.push(opt.to_string());
            }
            cmd.push("-o".into());
            cmd.push(output_str);
            cmd.push("-".into());
            cmds.push(cmd);
        }
        _ => {}
    }

    cmds
}

fn run_pipeline(cmds: &[Vec<String>]) -> anyhow::Result<Vec<(i32, String)>> {
    if cmds.is_empty() {
        return Ok(Vec::new());
    }

    let mut children: Vec<std::process::Child> = Vec::new();

    for (i, cmd) in cmds.iter().enumerate() {
        let stdin = if i == 0 {
            Stdio::null()
        } else {
            Stdio::from(children[i - 1].stdout.take().unwrap())
        };

        let child = Command::new(&cmd[0])
            .args(&cmd[1..])
            .stdin(stdin)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .with_context(|| format!("Failed to spawn: {}", cmd[0]))?;

        children.push(child);
    }

    let mut last_child = children.pop().unwrap();
    let last_output = last_child.wait_with_output()?;

    let mut results = Vec::new();
    for child in children.iter_mut() {
        let status = child.wait()?;
        let mut stderr = String::new();
        if let Some(ref mut e) = child.stderr {
            e.read_to_string(&mut stderr)?;
        }
        results.push((status.code().unwrap_or(-1), stderr));
    }
    results.push((
        last_output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&last_output.stderr).to_string(),
    ));

    Ok(results)
}

pub fn transcode_file(
    flac_file: &Path,
    output_dir: &Path,
    format: &Format,
) -> anyhow::Result<PathBuf> {
    let (sample_rate, bit_depth, channels) = get_flac_properties(flac_file)?;
    let resample = sample_rate > 48000 || bit_depth > 16;

    if channels > 2 {
        anyhow::bail!("Multichannel FLAC unsupported: {}", flac_file.display());
    }

    let needed_sample_rate = if resample {
        if sample_rate % 44100 == 0 {
            Some(44100)
        } else if sample_rate % 48000 == 0 {
            Some(48000)
        } else {
            anyhow::bail!(
                "Unknown sample rate {}Hz for {}",
                sample_rate,
                flac_file.display()
            );
        }
    } else {
        None
    };

    let stem = flac_file.file_stem().unwrap().to_string_lossy();
    let clean_stem = Regex::new(r#"[\?<>\\*\|":]"#)
        .unwrap()
        .replace_all(&stem, "_");
    let output_file = output_dir.join(format!("{}{}", clean_stem, format.encoder.ext));

    if let Some(parent) = output_file.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let cmds = build_pipeline_commands(
        format,
        resample,
        needed_sample_rate,
        flac_file,
        &output_file,
    );
    let results = run_pipeline(&cmds)?;

    let mut sigpipe_error = None;
    for (cmd, (code, stderr)) in cmds.iter().zip(results.iter()) {
        if *code != 0 {
            if *code == 141 || *code == -13 {
                sigpipe_error = Some(format!("{}: SIGPIPE", cmd.join(" ")));
            } else {
                anyhow::bail!("Transcode of {:?} failed: {}", flac_file, stderr);
            }
        }
    }
    if let Some(err) = sigpipe_error {
        anyhow::bail!("Transcode failed: {}", err);
    }

    tagging::copy_tags(flac_file, &output_file)?;

    let (ok, msg) = tagging::check_tags(&output_file, true);
    if !ok {
        anyhow::bail!("Tag check failed: {}", msg.unwrap_or_default());
    }

    Ok(output_file)
}

pub fn transcode_release(
    flac_dir: &Path,
    output_dir: &Path,
    format: &Format,
    torrent: &Torrent,
    group: &TorrentGroup,
) -> anyhow::Result<PathBuf> {
    let flac_dir = flac_dir.canonicalize()?;
    let output_dir = output_dir.canonicalize()?;
    let resample = needs_resampling(&flac_dir)?;

    if format.name == "FLAC" && !resample {
        return Ok(flac_dir);
    }

    let transcode_dir = output_dir.join(group.get_transcode_dirname(torrent, format));
    log::info!("    {}", transcode_dir.display());
    if transcode_dir.exists() {
        return Ok(transcode_dir);
    }
    std::fs::create_dir_all(&transcode_dir)?;

    let flac_files = find_files(&flac_dir, "flac")?;
    for flac_file in &flac_files {
        let relative = flac_file.strip_prefix(&flac_dir)?;
        let output_file_dir = transcode_dir.join(relative.parent().unwrap_or(Path::new("")));
        std::fs::create_dir_all(&output_file_dir)?;
        transcode_file(flac_file, &output_file_dir, format)?;
        let print_name = flac_file
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        log::info!("      Processing: {}", print_name);
    }

    let allowed_extensions = [
        "cue", "gif", "jpeg", "jpg", "log", "md5", "nfo", "pdf", "png", "sfv", "txt",
    ];
    for ext in &allowed_extensions {
        let files = find_files(&flac_dir, ext)?;
        for file in files {
            let relative = file.strip_prefix(&flac_dir)?;
            let dest_dir = transcode_dir.join(relative.parent().unwrap_or(Path::new("")));
            std::fs::create_dir_all(&dest_dir)?;
            std::fs::copy(&file, dest_dir.join(relative.file_name().unwrap()))?;
        }
    }

    Ok(transcode_dir)
}

pub fn make_torrent(
    input_dir: &Path,
    output_dir: &Path,
    tracker: &str,
    passkey: &str,
    source: Option<&str>,
) -> anyhow::Result<PathBuf> {
    let torrent_name = format!(
        "{}.torrent",
        input_dir.file_name().unwrap().to_string_lossy()
    );
    let torrent_path = output_dir.join(&torrent_name);

    if let Some(parent) = torrent_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let tracker_url = format!("{}{}/announce", tracker, passkey);

    let mut cmd = Command::new("mktorrent");
    cmd.arg("-p");
    if let Some(source) = source {
        cmd.arg("-s").arg(source);
    }
    cmd.arg("-a")
        .arg(&tracker_url)
        .arg("-o")
        .arg(&torrent_path)
        .arg(input_dir);

    let output = cmd.output().context("Failed to run mktorrent")?;
    if !output.status.success() {
        anyhow::bail!(
            "mktorrent failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    Ok(torrent_path)
}

pub fn transcode_commands_desc(
    format: &Format,
    resample: bool,
    sample_rate: Option<u32>,
) -> Vec<String> {
    let input = "input.flac";
    let output = format!("output{}", format.encoder.ext);
    let cmds = build_pipeline_commands(
        format,
        resample,
        sample_rate,
        Path::new(input),
        Path::new(&output),
    );
    cmds.iter().map(|cmd| cmd.join(" ")).collect()
}
