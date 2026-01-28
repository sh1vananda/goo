# Goo Logger (VLC)

## Install
- Copy `vlc/goo_logger.lua` into VLC's `lua/extensions` folder.
  - Windows: `%APPDATA%\vlc\lua\extensions`
  - macOS: `~/Library/Application Support/org.videolan.vlc/lua/extensions`
  - Linux: `~/.local/share/vlc/lua/extensions` or `~/.config/vlc/lua/extensions`
- Restart VLC, then open `View -> Extensions -> Goo Logger` to activate it.

## Log format
Each line is:

```
<UTC-ISO8601>|<absolute-path-or-uri>
```

Example:

```
2026-01-28T12:34:56Z|C:\Movies\Dune.2021.1080p.mkv
```

## Notes
- The default log file is `.goo_watch_log.txt` in VLC's user data directory.
- To change the log path, edit `LOG_PATH` near the top of `vlc/goo_logger.lua`.
