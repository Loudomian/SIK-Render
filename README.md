<p align="center">
  <picture>
    <source media="(prefers-color-scheme: dark)" srcset="app/assets/app-icon-dark.png">
    <img src="app/assets/app-icon-light.png" alt="SIK Render logo" width="96" height="96">
  </picture>
</p>

<h1 align="center">SIK Render</h1>

![License](https://img.shields.io/badge/license-MIT%20%2F%20Apache--2.0-blue.svg)
![Rust](https://img.shields.io/badge/Rust-stable-orange.svg)
![Vue 3](https://img.shields.io/badge/Vue.js-3.x-4FC08D.svg)
![Tauri](https://img.shields.io/badge/Tauri-2-24C8DB.svg)

SIK Render is a local desktop render manager for SIKFilm workflows. It manages Blender render jobs, FFmpeg transcodes, quick MP4 previews, and LAN render node monitoring.

## Features

- Blender render queue with pause, cancel, retry, and frame-range re-rendering.
- Quick MP4 jobs rendered directly by Blender with final-frame preview generation.
- FFmpeg transcode queue with automatic and manual jobs.
- Image/video previews, per-job logs, output templates, and persisted settings.
- LAN node discovery through mDNS with node progress, notes, and previews.

## Development

Requirements: Windows 10/11, Blender, FFmpeg, Bun, Rust stable, and Tauri 2 dependencies.

```powershell
bun install
bun run tauri:dev
```

Useful checks:

```powershell
bun run typecheck
cd src-tauri
cargo check
```

Build:

```powershell
bun run tauri:build
bun run tauri:bin
```

## LAN Nodes

Default node port: `47878`.

Default network reference: `192.168.1.1`. This is treated as a gateway/subnet reference for selecting a local `192.168.1.xxx` interface; the local machine does not need to use that exact IP.

Nodes appear automatically when devices are reachable on the same LAN and firewall access is allowed for the node port and mDNS.

## Data

Development builds store runtime files in the repository root. Production builds on Windows use:

```text
%APPDATA%\SIKFilm\Render\
```

Key files and directories:

- `sik-render.toml`: app settings
- `node-id.toml`: local node identity
- `sik-render.sqlite3`: render/transcode job database
- `jobs\blender\blender_job_0001_abcd\job.toml`: Blender job snapshot
- `jobs\blender\blender_job_0001_abcd\log\blender_<timestamp>.log`: Blender job logs
- `jobs\blender\blender_job_0001_abcd\preview.jpg`: Blender job preview
- `jobs\ffmpeg\ffmpeg_job_0001_abcd\job.toml`: FFmpeg job snapshot
- `jobs\ffmpeg\ffmpeg_job_0001_abcd\log\ffmpeg_<timestamp>.log`: FFmpeg job logs
- `logs\app\<version>\sikrender_<timestamp>.log`: app logs
- `nodes\peers\*.toml`: persisted LAN peer snapshots

## License

Licensed under either of:

- MIT License, see `LICENSE-MIT`
- Apache License, Version 2.0, see `LICENSE-APACHE-2.0`
