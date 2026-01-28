-- Goo Logger (Auto): VLC Lua interface that logs played media without manual activation.

local LOG_PATH = "C:\\Users\\cyber\\AppData\\Roaming\\vlc\\.goo_watch_log.txt"
local POLL_INTERVAL_US = 1000000 -- 1s
local running = false
local last_uri = nil

local function dir_sep()
    return package.config:sub(1, 1)
end

local function join_path(base, name)
    local sep = dir_sep()
    if base:sub(-1) == sep then
        return base .. name
    end
    return base .. sep .. name
end

local function default_log_path()
    local base = nil
    if vlc.config and vlc.config.userdatadir then
        local ok, value = pcall(vlc.config.userdatadir)
        if ok and value and value ~= "" then
            base = value
        end
    end

    if not base then
        base = os.getenv("APPDATA") or os.getenv("HOME") or "."
    end

    return join_path(base, ".goo_watch_log.txt")
end

local function ensure_log_path()
    if not LOG_PATH or LOG_PATH == "" then
        LOG_PATH = default_log_path()
    end
end

local function touch_log()
    ensure_log_path()
    local file, err = io.open(LOG_PATH, "a")
    if not file then
        vlc.msg.err("goo_logger_intf: failed to open log file: " .. tostring(err))
        return
    end
    file:close()
    vlc.msg.info("goo_logger_intf: logging to " .. LOG_PATH)
end

local function append_line(line)
    ensure_log_path()
    local file, err = io.open(LOG_PATH, "a")
    if not file then
        vlc.msg.err("goo_logger_intf: failed to open log file: " .. tostring(err))
        return
    end
    file:write(line)
    file:write("\n")
    file:close()
end

local function decode_uri(uri)
    if not uri or uri == "" then
        return nil
    end
    if vlc.strings and vlc.strings.decode_uri then
        local ok, value = pcall(vlc.strings.decode_uri, uri)
        if ok and value and value ~= "" then
            return value
        end
    end
    return uri
end

local function current_item_uri()
    local item = vlc.input.item()
    if not item then
        return nil
    end
    return decode_uri(item:uri())
end

local function log_if_needed()
    local uri = current_item_uri()
    if not uri then
        return
    end
    if not vlc.input.is_playing() then
        return
    end
    if uri ~= last_uri then
        last_uri = uri
        local timestamp = os.date("!%Y-%m-%dT%H:%M:%SZ")
        append_line(timestamp .. "|" .. uri)
    end
end

function descriptor()
    return {
        title = "Goo Logger (Auto)",
        version = "0.1",
        author = "goo",
        shortdesc = "Logs watched media to a file",
        description = "Background logger for goo that runs on startup.",
    }
end

function activate()
    running = true
    vlc.msg.info("goo_logger_intf: activated")
    touch_log()
    while running do
        log_if_needed()
        vlc.misc.mwait(vlc.misc.mdate() + POLL_INTERVAL_US)
    end
end

function deactivate()
    running = false
    vlc.msg.info("goo_logger_intf: deactivated")
end

function close()
    deactivate()
end
