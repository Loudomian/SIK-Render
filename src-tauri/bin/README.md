Place the packaged ffmpeg binary here.

Expected names:

- Windows: `ffmpeg.exe`
- macOS / Linux: `ffmpeg`

Build behavior:

- `src-tauri/tauri.conf.json` bundles `bin/**` as app resources
- runtime lookup prefers bundled resources, then falls back to this workspace `bin/` folder during development
