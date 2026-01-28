# Goo Logger (VLC)

## Auto-start install (recommended)
This runs in the background every time VLC starts with no manual activation.

- Copy `vlc/goo_logger_intf.lua` into VLC's `lua/intf` folder.
  - Windows: `%APPDATA%\vlc\lua\intf`
  - macOS: `~/Library/Application Support/org.videolan.vlc/lua/intf`
  - Linux: `~/.local/share/vlc/lua/intf` or `~/.config/vlc/lua/intf`
- In VLC: `Tools -> Preferences` and set `Show settings` to `All`.
- Go to `Interface -> Main interfaces`:
  - Add `luaintf` to `Extra interface modules`.
  - In the `Lua` section below, set `Lua interface` to `goo_logger_intf`.
- Restart VLC.


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
- To change the log path, edit `LOG_PATH` near the top of `vlc/goo_logger_intf.lua`.
