# GOO

A minimal VLC watch history tracker that automatically logs your viewing activity and enriches it with metadata from TMDB.

## Features

- **Automatic Logging**: VLC Lua script logs every video you play
- **TMDB Integration**: Fetches movie metadata, posters, and links
- **Smart Title Cleaning**: Removes quality tags, codecs, and other filename junk
- **Tauri Desktop App**: Fast, native desktop interface
- **CLI Tool**: Command-line interface for processing logs
- **Minimal UI**: Clean, dark, focused design

## Architecture

```
┌─────────────────┐
│  VLC Player     │  Logs playback to text file
└────────┬────────┘
         │
         v
┌─────────────────┐
│  .goo_watch_log │  Raw log file (timestamp|file path)
└────────┬────────┘
         │
         v
┌─────────────────┐
│  Rust Backend   │  Parses logs, cleans titles, fetches TMDB
└────────┬────────┘
         │
         v
┌─────────────────┐
│  Tauri App      │  Desktop UI displaying watch history
└─────────────────┘
```

## Prerequisites

- **VLC Media Player**: For automatic logging
- **Rust**: `cargo` toolchain for building
- **Node.js**: For the Tauri frontend
- **TMDB API Key**: [Get one here](https://www.themoviedb.org/settings/api) (free)

## Installation

### 1. VLC Lua Script Setup

Copy the logger script to VLC's Lua interface directory:

```bash
# Windows
copy vlc\goo_logger_intf.lua %APPDATA%\vlc\lua\intf\

# macOS/Linux
cp vlc/goo_logger_intf.lua ~/.local/share/vlc/lua/intf/
```

Configure VLC:
1. Open VLC preferences (`Tools → Preferences → All`)
2. Navigate to `Interface → Main interfaces`
3. Check "Lua interpreter"
4. Set `Lua Interface` to: `goo_logger_intf`

Or manually edit `vlcrc`:
```ini
lua-intf=goo_logger_intf
extraintf=luaintf
```

**Important**: Ensure `lua=0` is NOT in your vlcrc (it disables Lua).

### 2. Set TMDB API Key

```bash
# Windows (PowerShell)
$env:TMDB_API_KEY = "your_api_key_here"

# macOS/Linux
export TMDB_API_KEY="your_api_key_here"
```

Or enter it directly in the app's settings panel.

### 3. Build the Application

```bash
# Install frontend dependencies
cd frontend
npm install
cd ..

# Build and run Tauri app
cd src-tauri
cargo tauri dev
```

For production build:
```bash
cargo tauri build
```

## Usage

### Desktop App

1. Launch the app: `cargo tauri dev`
2. Click the **⋮** icon to open settings (optional)
3. Enter your TMDB API key if not set as environment variable
4. Click **🔄** to refresh and load your watch history
5. Browse your movies with posters, titles, and TMDB links

**Settings**:
- **Log Path**: Auto-detected from `%APPDATA%\vlc\.goo_watch_log.txt`
- **Cache Path**: Auto-detected (stores TMDB results)
- **TMDB API Key**: Persisted in browser localStorage

### CLI Tool

```bash
# Clean and enrich watch history (outputs JSON)
cargo run enrich

# Clean titles without TMDB enrichment
cargo run clean

# Specify custom paths
cargo run enrich /path/to/log.txt /path/to/cache.json
```

### Log Format

The VLC script writes to `.goo_watch_log.txt`:
```
2026-01-28T12:34:56Z|file:///C:/Movies/Blade.Runner.2049.1080p.mkv
2026-01-28T13:45:00Z|file:///C:/Movies/Dune.Part.Two.2024.2160p.mkv
```

Each line: `ISO8601_timestamp|file_uri`

## How Title Cleaning Works

Example transformation:
```
Amores.Perros.2000.1080p.BluRay.x264.AAC5.1-[YTS.MX].mp4
↓
Amores Perros
```

The cleaning process:
1. Remove bracketed content: `[YTS.MX]`, `(2000)`, etc.
2. Remove codec/quality tags: `AAC5.1`, `1080p`, `x264`, etc.
3. Replace separators (`.`, `_`, `-`) with spaces
4. Remove years (1900-2099)
5. Remove standalone numbers

Regex patterns removed:
- Quality: `480p`, `720p`, `1080p`, `2160p`, `4k`, etc.
- Codecs: `x264`, `x265`, `h264`, `h265`, `hevc`, `aac`, `dts`, etc.
- Sources: `bluray`, `webrip`, `web-dl`, `hdr`, `dvdrip`, etc.
- Release groups: `yts`, `yify`, `rarbg`, `etrg`, `pahe`, etc.

## Features in Detail

### Deduplication
The app automatically filters duplicate entries, keeping only the most recent viewing of each unique movie.

### TMDB Caching
Movie metadata is cached locally to reduce API calls. Delete `.goo_cache.json` to force refresh.

### Minimal UI
- Pure black background (`#000000`)
- Sharp corners (6px radius)
- Tiny cards (140px minimum width)
- No gradients, no shadows
- Fast transitions (100-150ms)
- Hover to see full titles (1s delay)

## Project Structure

```
goo/
├── src/                 # Rust library
│   ├── lib.rs          # Log parsing, title cleaning
│   ├── app.rs          # Main application logic
│   ├── tmdb.rs         # TMDB API client
│   ├── enrich.rs       # Enrichment with caching
│   └── main.rs         # CLI entry point
├── src-tauri/          # Tauri desktop app
│   └── src/main.rs     # Tauri backend
├── frontend/           # React frontend
│   ├── src/
│   │   ├── App.tsx     # Main UI component
│   │   └── styles.css  # Minimal dark theme
│   └── package.json
├── vlc/
│   ├── goo_logger_intf.lua  # VLC interface script
│   └── README.md            # VLC setup instructions
└── README.md
```

## Troubleshooting

### VLC Script Not Running

1. Check `Tools → Messages` (Ctrl+M) with verbosity 2
2. Verify `lua=0` is NOT in `vlcrc`
3. Confirm script is in the correct directory:
   - Windows: `%APPDATA%\vlc\lua\intf\`
   - macOS: `~/Library/Application Support/org.videolan.vlc/lua/intf/`
   - Linux: `~/.local/share/vlc/lua/intf/`

### TMDB Errors

- Ensure API key is set (environment variable or app settings)
- Check internet connection
- Verify API key is valid at [TMDB settings](https://www.themoviedb.org/settings/api)

### Missing Metadata

- Delete cache: `rm ~/.goo_cache.json` (or `%APPDATA%\vlc\.goo_cache.json`)
- Click refresh to re-fetch from TMDB
- Check if title cleaning is too aggressive (adjust regex in `src/lib.rs`)

### Empty History

- Play a video in VLC
- Check if log file exists: `%APPDATA%\vlc\.goo_watch_log.txt`
- Verify VLC script is loaded (check Messages window)

## Development

```bash
# Run tests
cargo test

# Format code
cargo fmt

# Lint
cargo clippy

# Frontend dev server
cd frontend && npm run dev

# Tauri dev with hot reload
cd src-tauri && cargo tauri dev
```

## License

MIT

## Credits

- TMDB for movie metadata
- VLC for the amazing media player and Lua API
- Tauri for the lightweight desktop framework
