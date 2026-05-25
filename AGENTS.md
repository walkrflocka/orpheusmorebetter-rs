# AGENTS.md

## Build & Verify

```bash
cargo check          # fast compile check — run after every change
cargo build          # full build
cargo run -- --help  # shows CLI flags
cargo clippy         # lint — no clippy.toml, defaults apply
cargo fmt --check    # format check — no rustfmt.toml, defaults apply
```

No test suite exists yet. No CI. No pre-commit hooks.

## Reference Implementation

The Python tool at `/workspaces/orpheusmorebetter` is the authoritative source for behavior. When Rust implementation details are unclear, check the Python equivalent first:

- `orpheusmorebetter` (CLI entrypoint)
- `services/whatapi.py` (API client)
- `services/transcode.py` (pipeline)
- `services/tagging.py` (metadata)

## Critical Domain Rules

**Transcoding pipeline order matters**: decode → resample (if needed) → encode. Stderr must be captured from every process in the chain.

**Resampling logic**: Only resample if `bit_depth > 16` OR `sample_rate > 48000`. Target rate is 44100 Hz for 44.1/88.2/176.4 kHz sources, 48000 Hz for 48/96/192 kHz sources. Multichannel (>2) is rejected entirely.

**Edition matching**: Transcodes only happen within the same edition (media + remaster year/title/label/catalog#). Pre-emphasis in remaster title blocks transcoding.

**API rate limit**: Max 5 requests per 10 seconds. `WhatAPI` enforces this via a sliding window in `rate_limit()` — called automatically by `request_ajax()`.

## Current State

Project compiles with warnings (unused fields, dead code). CLI works; transcoding pipeline implemented but untested end-to-end.

**Module structure**:
```
src/
  main.rs              # CLI (clap) + main processing loop
  config.rs            # INI config loading (rust-ini)
  cache.rs             # SQLite cache (rusqlite)
  models/
    mod.rs
    artist.rs          # Artist struct
    error.rs           # TranscodeError, ApiError enums
    format.rs          # Format, Encoder (FLAC, MP3 V0, MP3 320)
    torrent.rs         # Torrent struct + allowed_transcodes()
    torrentgroup.rs    # TorrentGroup + API response deserialization
  services/
    mod.rs
    whatapi.rs         # WhatAPI: login, rate-limit, AJAX, upload, scraping
    transcode.rs       # Pipeline construction + execution, mktorrent
    tagging.rs         # Tag copy (FLAC→FLAC, FLAC→MP3) + validation
```

**Known issues**:
- Cookie jar inspection: reqwest follows post-login 302 internally, `Set-Cookie` not visible on final response. Need to inspect jar or intercept redirect.
- Per-format output/torrent dir overrides not wired up (config.rs has stubs)
- No multithreading (Python also has it disabled)

## Coding Conventions

- **Rust 2024 edition**, `unsafe_code = "forbid"`
- **Errors**: `thiserror` for type definitions, `anyhow` for propagation. No `unwrap()`/`expect()` in non-test code.
- **Ownership**: Prefer `String` over `&'a str` where lifetimes cause friction.
- **Serde**: Use for all API request/response types.
- **Async**: Single-threaded tokio runtime (`current_thread` flavor).

## External Tools

These are invoked as subprocesses. Exact commands matter:

| Tool | Command |
|------|---------|
| flac (decode) | `flac -dcs -- {input.flac}` → stdout |
| sox (resample) | `sox {input.flac} -G -b 16 -t wav - rate -v -L {target_rate} dither` → stdout |
| lame V0 | `lame -S -V 0 --vbr-new --ignore-tag-errors - {out.mp3}` |
| lame 320 | `lame -S -h -b 320 --ignore-tag-errors - {out.mp3}` |
| mktorrent | `mktorrent -p [-s {source}] -a {announce_url}/{passkey}/announce -o {output} {dir}` |
| metaflac | `metaflac --show-sample-rate --show-bps --show-channels {file}` |

## Config

INI format at `~/.orpheusmorebetter/config` (shared with Python tool). See CLAUDE.md §7 for schema.

## Cache

SQLite database at `~/.orpheusmorebetter/cache`. Single table `seen` with `torrent_id TEXT PRIMARY KEY`.
