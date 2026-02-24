# orpheusmorebetter-rs

## 1. Project Purpose

Orpheus.network is a private music tracker. Users seed FLACs; this tool finds which lossy formats (MP3 V0, MP3 320) are missing from a torrent group and auto-transcodes + uploads them. This is a Rust rewrite of the mature Python tool `orpheusmorebetter`.

## 2. Reference Implementation

The canonical Python implementation lives at `~/repos/orpheusmorebetter`. Defer to it for authoritative behaviour, except when told to change implementation.

Key Python files:
- `orpheusmorebetter` — CLI entrypoint
- `services/whatapi.py` — Gazelle API client
- `services/transcode.py` — pipeline construction and execution
- `services/tagging.py` — FLAC/MP3 tag read/write
- `models/` — Torrent, TorrentGroup, Format types

## 3. Domain Concepts

**Formats**
- `FLAC` — lossless
- `MP3 V0` — VBR: `lame -V 0 --vbr-new`
- `MP3 320` — CBR: `lame -b 320`

**TorrentGroup**: An album on Orpheus. Contains multiple `Torrent` entries, one per format/edition.

**Edition matching**: Same media + remaster year/title/label/catalog# = same edition. Transcodes only happen within an edition.

**Allowed transcodes**: A torrent can be transcoded only if no pre-emphasis flag appears in the remaster title. Existing formats in the group edition are subtracted from the candidate set to produce the "needed" set.

**24-bit / high sample rate handling**:
- Must resample to 16-bit
- Target rate: 44100 Hz (for 44.1/88.2/176.4 kHz sources) or 48000 Hz (for 48/96/192 kHz sources)
- If already 16-bit and ≤48 kHz, no resampling needed
- Multichannel (>2 channels) is rejected entirely

## 4. External CLI Tools

These are invoked as subprocesses and chained as stdin/stdout pipelines. **Stderr must be captured from every process in the chain.**

| Tool | Command |
|------|---------|
| **flac** (decode, no resample) | `flac -dcs -- {input.flac}` → raw audio to stdout |
| **sox** (decode + resample) | `sox {input.flac} -G -b 16 -t wav - rate -v -L {target_rate} dither` → stdout |
| **lame V0** | `lame -S -V 0 --vbr-new --ignore-tag-errors - {out.mp3}` (reads stdin) |
| **lame 320** | `lame -S -h -b 320 --ignore-tag-errors - {out.mp3}` (reads stdin) |
| **mktorrent** | `mktorrent -p [-s {source}] -a {announce_url}/{passkey}/announce -o {output} {dir}` |

## 5. Transcoding Pipeline (step order)

1. Discover all `.flac` files in source directory
2. Read FLAC metadata (sample rate, bit depth, channels)
3. Decide resampling (needed if `bit_depth > 16` OR `sample_rate > 48000`)
4. Determine target sample rate (44100 or 48000)
5. For each needed format: build pipeline (decoder | [resampler] | encoder), execute, capture all stderr
6. Copy metadata tags (FLAC→FLAC direct copy; FLAC→MP3 via ID3 mapping)
7. Copy supplementary files (`.cue`, `.log`, `.nfo`, `.md5`, `.sfv`, `.txt`, images)
8. Create torrent with `mktorrent`
9. Upload via Gazelle API (`upload` action)
10. Record processed torrent ID in persistent cache

## 6. Gazelle API

Base URL from config (`api` key). All JSON endpoints: `GET/POST {base}/ajax.php?action={action}`.

**Auth**: POST login to `login.php`; subsequent requests use session cookie. `authkey` and `passkey` retrieved from `index` action after login.

**Rate limit**: max 5 requests per 10 seconds.

**Key actions**:
| Action | Method | Notes |
|--------|--------|-------|
| `index` | GET | Returns authkey, passkey, user_id |
| `torrent` (id=N) | GET | Single torrent metadata |
| `torrentgroup` (id=N) | GET | Group + all torrents |
| `upload` | POST | Upload new torrent to existing group |
| `torrents.php?action=download&id=N&authkey=...&torrent_pass=...` | GET | Download .torrent file |

HTML scraping of `torrents.php` (snatched/uploaded pages) and `better.php` for candidate discovery.

## 7. Configuration

INI format at `~/.orpheusmorebetter/config` (same path reused for the Rust tool):

```ini
[orpheus]
username =
password =
data_dir =          # semicolon-separated if multiple
output_dir =        # defaults to data_dir
torrent_dir =
formats = flac, v0, 320
media = cd, vinyl, web, sacd, soundboard, dvd, dat, blu-ray
tracker = https://home.opsfet.ch/
api = https://orpheus.network/
mode = both         # snatched|uploaded|both|seeding|all|none
source = OPS
24bit_behaviour = 0 # 0=ignore, 1=prompt, 2=auto-fix
```

Per-format output/torrent dir overrides also supported (`output_dir_v0`, `torrent_dir_320`, etc.).

## 8. Module Structure

Current:
```
src/
  main.rs                     # CLI entrypoint; will use clap
  app_config.rs               # AppConfig, Format enum, LosslessMediaSources enum
  whatapi.rs                  # WhatAPI struct: login, request_ajax, get_keys
  ajax.rs                     # AjaxSession: ajax.php wrapper (has bugs to fix)
  extensions/
    mod.rs
    whatapi_middleware.rs     # reqwest-middleware for auth injection (has bugs)
```

Intended future modules (mirror Python structure):
- `transcode.rs` — pipeline construction and execution
- `tagging.rs` — FLAC/MP3 tag read/write (likely via `symphonia` or `metaflac` + `id3`)
- `models/` — Torrent, TorrentGroup, Format, Encoder types
- `cache.rs` — persistent set of processed torrent IDs (replace Python pickle)

## 9. Current State & Known Issues

Project is **pre-alpha and does not compile**. Known bugs:

| Location | Issue |
|----------|-------|
| `ajax.rs:29` | Undefined `url` — should be `self.url` |
| `ajax.rs:33` | `post()` body is missing |
| `whatapi_middleware.rs:27` | Incomplete `req.query_mut()` expression |
| `whatapi.rs:53` | `unwrap()` on optional TOTP panics if absent |
| `main.rs` | Hardcoded placeholder credentials, no CLI arg parsing yet |
| `whatapi.rs` `get_keys()` | Empty stub |

## 10. Coding Conventions

- **Edition**: Rust 2024
- **Safety**: `unsafe_code = "forbid"` — no unsafe Rust
- **Async**: `tokio`; currently uses `block_on`, add tokio as explicit dep when needed
- **Errors**: `thiserror` for error type definitions; `anyhow` for application-level propagation
- **No panics in library code**: no `unwrap()`/`expect()` in non-test paths; use `?` and proper error types
- **Ownership**: current code uses `&'a str` lifetimes in structs — prefer `String` where lifetimes cause friction (config fields, API responses)
- **Serde**: use for all API request/response types
- **CLI**: `clap` (derive feature) is in Cargo.toml but commented out — enable when implementing CLI
