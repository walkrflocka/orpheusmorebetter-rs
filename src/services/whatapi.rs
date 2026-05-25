use std::collections::HashSet;
use std::collections::VecDeque;
use std::path::Path;
use std::time::{Duration, Instant};

use anyhow::Context;
use regex::Regex;
use reqwest::{Client, ClientBuilder, Url};
use serde::Deserialize;

use crate::config::{AppConfig, LOSSLESS_MEDIA, MEDIA_SEARCH_MAP};
use crate::models::format::Format;
use crate::models::torrent::Torrent;
use crate::models::torrentgroup::{TorrentGroup, TorrentGroupApiResponse};

const RATE_LIMIT_WINDOW: Duration = Duration::from_secs(10);
const RATE_LIMIT_MAX_REQUESTS: usize = 5;

#[derive(Deserialize)]
struct AjaxResponse<T> {
    status: String,
    response: T,
    error: Option<String>,
}

#[derive(Deserialize)]
struct IndexResponse {
    id: u32,
    authkey: String,
    passkey: String,
}

pub struct WhatAPI {
    client: Client,
    endpoint: Url,
    pub user_id: u32,
    pub authkey: String,
    pub passkey: String,
    request_timestamps: VecDeque<Instant>,
}

impl WhatAPI {
    pub async fn new(config: &AppConfig, totp: Option<String>) -> anyhow::Result<Self> {
        let endpoint: Url = config
            .api_endpoint
            .parse()
            .context("Invalid API endpoint URL")?;

        let mut headers = reqwest::header::HeaderMap::new();
        headers.insert("User-Agent", "orpheusmorebetter".parse().unwrap());

        let client = ClientBuilder::new()
            .cookie_store(true)
            .default_headers(headers)
            .build()
            .context("Failed to build HTTP client")?;

        Self::login(
            &client,
            &config.username,
            &config.password,
            &endpoint,
            totp.as_deref(),
        )
        .await?;

        let keys = Self::get_keys(&client, &endpoint).await?;
        log::info!("Logged into Orpheus.");

        Ok(WhatAPI {
            client,
            endpoint,
            user_id: keys.id,
            authkey: keys.authkey,
            passkey: keys.passkey,
            request_timestamps: VecDeque::with_capacity(RATE_LIMIT_MAX_REQUESTS),
        })
    }

    async fn login(
        client: &Client,
        username: &str,
        password: &str,
        endpoint: &Url,
        mfa: Option<&str>,
    ) -> anyhow::Result<()> {
        let url = endpoint.join("login.php").context("Invalid login URL")?;

        let mut form = vec![
            ("username", username),
            ("password", password),
            ("login", "Log in"),
        ];
        if let Some(mfa) = mfa {
            form.push(("mfa", mfa));
        }

        let res = client.post(url).form(&form).send().await?;
        let status = res.status();
        if status.is_client_error() || status.is_server_error() {
            anyhow::bail!("Login failed with status {}", status);
        }

        Ok(())
    }

    async fn get_keys(client: &Client, endpoint: &Url) -> anyhow::Result<IndexResponse> {
        let url = endpoint.join("ajax.php").context("Invalid ajax URL")?;
        let res = client.get(url).query(&[("action", "index")]).send().await?;

        let status = res.status();
        if !status.is_success() {
            anyhow::bail!("Index request failed with status {}", status);
        }

        let body: AjaxResponse<IndexResponse> = res.json().await?;
        if body.status != "success" {
            anyhow::bail!(
                "Login failed: {}",
                body.error.unwrap_or_else(|| "unknown error".to_string())
            );
        }

        Ok(body.response)
    }

    async fn rate_limit(&mut self) {
        let now = Instant::now();
        let cutoff = now - RATE_LIMIT_WINDOW;

        while let Some(&front) = self.request_timestamps.front() {
            if front < cutoff {
                self.request_timestamps.pop_front();
            } else {
                break;
            }
        }

        if self.request_timestamps.len() >= RATE_LIMIT_MAX_REQUESTS {
            if let Some(&oldest) = self.request_timestamps.front() {
                let wait_until = oldest + RATE_LIMIT_WINDOW;
                if wait_until > now {
                    tokio::time::sleep(wait_until - now).await;
                }
            }
        }

        self.request_timestamps.push_back(Instant::now());
    }

    pub async fn request_ajax_get<T: for<'de> Deserialize<'de>>(
        &mut self,
        action: &str,
        extra_params: &[(&str, &str)],
    ) -> anyhow::Result<T> {
        self.rate_limit().await;

        let url = self.endpoint.join("ajax.php").context("Invalid ajax URL")?;
        let mut params: Vec<(&str, &str)> = vec![("action", action), ("auth", &self.authkey)];
        params.extend_from_slice(extra_params);

        let res = self.client.get(url).query(&params).send().await?;
        let status = res.status();
        if !status.is_success() {
            anyhow::bail!("AJAX GET {} failed with status {}", action, status);
        }

        let body: AjaxResponse<T> = res.json().await?;
        if body.status != "success" {
            anyhow::bail!(
                "AJAX error ({}): {}",
                action,
                body.error.unwrap_or_default()
            );
        }

        Ok(body.response)
    }

    pub async fn request_ajax_post(
        &mut self,
        action: &str,
        mut form: Vec<(&str, String)>,
    ) -> anyhow::Result<serde_json::Value> {
        self.rate_limit().await;

        let url = self.endpoint.join("ajax.php").context("Invalid ajax URL")?;
        form.push(("auth", self.authkey.clone()));

        let res = self
            .client
            .post(url)
            .query(&[("action", action)])
            .form(&form)
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            anyhow::bail!("AJAX POST {} failed with status {}", action, status);
        }

        let body: AjaxResponse<serde_json::Value> = res.json().await?;
        if body.status != "success" {
            anyhow::bail!(
                "AJAX error ({}): {}",
                action,
                body.error.unwrap_or_default()
            );
        }

        Ok(body.response)
    }

    pub async fn get_html(&mut self, url: &str) -> anyhow::Result<String> {
        self.rate_limit().await;

        let res = self
            .client
            .get(url)
            .query(&[("auth", self.authkey.as_str())])
            .send()
            .await?;

        Ok(res.text().await?)
    }

    pub async fn get_torrent_group(&mut self, group_id: u64) -> anyhow::Result<TorrentGroup> {
        let id_str = group_id.to_string();
        let resp: TorrentGroupApiResponse = self
            .request_ajax_get("torrentgroup", &[("id", &id_str)])
            .await?;
        Ok(TorrentGroup::from_response(resp))
    }

    pub async fn upload(
        &mut self,
        group: &TorrentGroup,
        torrent: &Torrent,
        torrent_file_path: &Path,
        format: &Format,
        description: &[String],
    ) -> anyhow::Result<()> {
        self.rate_limit().await;

        let file_bytes = tokio::fs::read(torrent_file_path).await?;
        let file_part = reqwest::multipart::Part::bytes(file_bytes)
            .file_name("1.torrent")
            .mime_str("application/x-bittorrent")?;

        let mut form = reqwest::multipart::Form::new()
            .part("file_input", file_part)
            .text("type", "0")
            .text("groupid", group.id.to_string())
            .text("remaster", "1")
            .text(
                "remaster_year",
                torrent.remaster_year.unwrap_or(0).to_string(),
            )
            .text("remaster_title", torrent.remaster_title.clone())
            .text(
                "remaster_record_label",
                torrent.remaster_record_label.clone(),
            )
            .text(
                "remaster_catalogue_number",
                torrent.remaster_catalogue_number.clone(),
            )
            .text("format", format.name.to_string())
            .text("bitrate", format.encoding.to_string())
            .text("media", torrent.media.clone())
            .text("auth", self.authkey.clone());

        if !description.is_empty() {
            form = form.text("release_desc", description.join("\n"));
        }

        let url = self.endpoint.join("ajax.php").context("Invalid ajax URL")?;
        let res = self
            .client
            .post(url)
            .query(&[("action", "upload")])
            .multipart(form)
            .send()
            .await?;

        let status = res.status();
        if !status.is_success() {
            anyhow::bail!("Upload failed with status {}", status);
        }

        let body: AjaxResponse<serde_json::Value> = res.json().await?;
        if body.status != "success" {
            anyhow::bail!("Upload error: {}", body.error.unwrap_or_default());
        }

        Ok(())
    }

    pub async fn set_24bit(&mut self, torrent: &Torrent) -> anyhow::Result<()> {
        self.rate_limit().await;

        let url = self
            .endpoint
            .join(&format!("torrents.php?action=edit&id={}", torrent.id))?;

        let form = vec![
            ("submit", "true".to_string()),
            ("type", "1".to_string()),
            ("action", "takeedit".to_string()),
            ("torrentid", torrent.id.to_string()),
            ("media", torrent.media.clone()),
            ("format", torrent.format.clone()),
            ("bitrate", "24bit Lossless".to_string()),
            ("release_desc", torrent.description.clone()),
            ("remaster", "on".to_string()),
            (
                "remaster_year",
                torrent.remaster_year.unwrap_or(0).to_string(),
            ),
            ("remaster_title", torrent.remaster_title.clone()),
            (
                "remaster_record_label",
                torrent.remaster_record_label.clone(),
            ),
            (
                "remaster_catalogue_number",
                torrent.remaster_catalogue_number.clone(),
            ),
        ];

        let res = self.client.post(url).form(&form).send().await?;
        if !res.status().is_success() {
            anyhow::bail!("set_24bit failed with status {}", res.status());
        }

        Ok(())
    }

    pub async fn crawl_torrents_php(
        &mut self,
        crawl_type: &str,
        media_params: &[String],
        skip: &HashSet<String>,
    ) -> anyhow::Result<Vec<(u64, u64)>> {
        log::info!("Finding {} torrents", crawl_type);
        let base_url = format!(
            "{}torrents.php?type={}&userid={}&format=FLAC",
            self.endpoint, crawl_type, self.user_id
        );

        let re = Regex::new(r"torrents\.php\?id=(\d+)&torrentid=(\d+)")?;
        let mut candidates = Vec::new();

        for mp in media_params {
            let mut page = 1u32;
            loop {
                let url = format!("{}{}&page={}", base_url, mp, page);
                let html = self.get_html(&url).await?;

                for cap in re.captures_iter(&html) {
                    let group_id: u64 = cap[1].parse()?;
                    let torrent_id: u64 = cap[2].parse()?;

                    if !skip.contains(&torrent_id.to_string()) {
                        candidates.push((group_id, torrent_id));
                    }
                }

                if !html.contains(&format!("page={}", page + 1)) {
                    break;
                }
                page += 1;
            }
        }

        Ok(candidates)
    }

    pub async fn get_candidates(
        &mut self,
        mode: &str,
        skip: &HashSet<String>,
        media: &HashSet<String>,
    ) -> anyhow::Result<Vec<(u64, u64)>> {
        let lossless_media: HashSet<String> =
            LOSSLESS_MEDIA.iter().map(|s| s.to_string()).collect();

        let media_params: Vec<String> = if media == &lossless_media {
            vec![String::new()]
        } else {
            media
                .iter()
                .filter_map(|m| {
                    MEDIA_SEARCH_MAP
                        .iter()
                        .find(|(k, _)| *k == m.as_str())
                        .map(|(_, v)| format!("&media={}", v))
                })
                .collect()
        };

        let mut candidates = Vec::new();

        if mode == "snatched" || mode == "both" || mode == "all" {
            candidates.extend(
                self.crawl_torrents_php("snatched", &media_params, skip)
                    .await?,
            );
        }

        if mode == "uploaded" || mode == "both" || mode == "all" {
            candidates.extend(
                self.crawl_torrents_php("uploaded", &media_params, skip)
                    .await?,
            );
        }

        if mode == "seeding" || mode == "all" {
            log::info!("Using better.php to find Seeding");
            let url = format!(
                "{}better.php?method=transcode&filter=seeding",
                self.endpoint
            );
            let html = self.get_html(&url).await?;
            let re = Regex::new(r"torrents\.php\?groupId=(\d+)&torrentid=(\d+)#\d+")?;
            for cap in re.captures_iter(&html) {
                let group_id: u64 = cap[1].parse()?;
                let torrent_id: u64 = cap[2].parse()?;
                if !skip.contains(&torrent_id.to_string()) {
                    candidates.push((group_id, torrent_id));
                }
            }
        }

        Ok(candidates)
    }

    pub fn release_url(&self, group: &TorrentGroup, torrent: &Torrent) -> String {
        format!(
            "{}torrents.php?id={}&torrentid={}#torrent{}",
            self.endpoint, group.id, torrent.id, torrent.id
        )
    }
}
