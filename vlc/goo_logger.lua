-- Goo Logger: minimal VLC Lua extension that appends watched media to a log file.
-- Install in VLC's lua/extensions/ folder and activate once per session.

local LOG_PATH = nil
local current_uri = nil
local logged_for_current = false

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

local function append_line(line)
    ensure_log_path()
    local file, err = io.open(LOG_PATH, "a")
    if not file then
        vlc.msg.err("goo_logger: failed to open log file: " .. tostring(err))
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
    local uri = item:uri()
    return decode_uri(uri)
end

local function update_current_uri()
    local uri = current_item_uri()
    if not uri then
        return
    end
    if uri ~= current_uri then
        current_uri = uri
        logged_for_current = false
    end
end

local function log_if_needed()
    update_current_uri()
    if logged_for_current or not current_uri then
        return
    end
    if not vlc.input.is_playing() then
        return
    end
    local timestamp = os.date("!%Y-%m-%dT%H:%M:%SZ")
    append_line(timestamp .. "|" .. current_uri)
    logged_for_current = true
end

function descriptor()
    return {
        title = "Goo Logger",
        version = "0.1",
        author = "goo",
        shortdesc = "Logs watched media to a file",
        description = "Append the current media item to a hidden log file for goo.",
        capabilities = { "input-listener" },
    }
end

function activate()
    vlc.msg.dbg("goo_logger: activated")
    log_if_needed()
end

function deactivate()
    vlc.msg.dbg("goo_logger: deactivated")
end

function close()
    vlc.deactivate()
end

function input_changed()
    log_if_needed()
end

function playing_changed()
    log_if_needed()
end
