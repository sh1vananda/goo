# Goo Logger (VLC)

## Auto-start install (recommended)
This runs in the background every time VLC starts with no manual activation.

1. Copy `vlc/goo_logger_intf.lua` into VLC's `lua/intf` folder:
   - Windows: `%APPDATA%\vlc\lua\intf\`
   - macOS: `~/Library/Application Support/org.videolan.vlc/lua/intf/`
   - Linux: `~/.local/share/vlc/lua/intf/` or `~/.config/vlc/lua/intf/`

2. Create the `intf` folder if it doesn't exist.

3. In VLC: `Tools -> Preferences` and set `Show settings` to `All`.

4. Go to `Interface -> Main interfaces`:
   - Check the box OR add `luaintf` to `Extra interface modules`.

5. Expand `Main interfaces` and click on `Lua`:
   - Set `Lua interface` to `goo_logger_intf` (without the `.lua` extension).

6. Save and restart VLC.

## Verifying it works

1. Open VLC and go to `Tools -> Messages` (or press `Ctrl+M`).
2. Set verbosity to `2` (or Debug).
3. You should see messages like:
   ```
   lua interface: goo_logger_intf: starting...
   lua interface: goo_logger_intf: logging to C:\Users\...\vlc\.goo_watch_log.txt
   lua interface: goo_logger_intf: activated and logging to ...
   ```
4. Play a video — you should see it logged in the file.

## Troubleshooting

- **Script not running?** 
  - Make sure the filename is exactly `goo_logger_intf.lua`
  - Make sure you set `Lua interface` to `goo_logger_intf` (not the full path)
  - Check `Tools -> Messages` for errors starting with "lua"
  
- **No log file appearing?**
  - Set a fixed path at the top of the script: `local LOG_PATH = "C:\\path\\to\\your\\log.txt"`
  - Make sure the target folder exists and is writable

- **Old extension interfering?**
  - Remove any `goo_logger.lua` file from `%APPDATA%\vlc\lua\extensions\`

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
- To change the log path, edit `LOG_PATH` near the top of `goo_logger_intf.lua`.
- The script polls every 1 second for playback changes.
