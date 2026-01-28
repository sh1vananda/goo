-- Goo Logger: VLC Lua interface script that logs played media automatically.
-- Place in: %APPDATA%\vlc\lua\intf\goo_logger_intf.lua
-- Configure: lua-intf=goo_logger_intf and extraintf=luaintf in vlcrc

-- Configuration
local LOG_PATH = nil  -- nil = auto-detect, or set a fixed path like "C:\\logs\\vlc_watch.txt"
local POLL_INTERVAL_US = 1000000  -- 1 second in microseconds

-- State
local last_uri = nil
local log_file_ok = false

-- Utility functions
local function get_log_path()
    if LOG_PATH and LOG_PATH ~= "" then
        return LOG_PATH
    end
    
    -- Try to get VLC's user data directory
    if vlc.config and vlc.config.userdatadir then
        local ok, userdir = pcall(vlc.config.userdatadir)
        if ok and userdir and userdir ~= "" then
            local sep = package.config:sub(1, 1)
            if userdir:sub(-1) ~= sep then
                userdir = userdir .. sep
            end
            return userdir .. ".goo_watch_log.txt"
        end
    end
    
    -- Fallback to APPDATA or HOME
    local base = os.getenv("APPDATA") or os.getenv("HOME") or "."
    local sep = package.config:sub(1, 1)
    if base:sub(-1) ~= sep then
        base = base .. sep
    end
    return base .. ".goo_watch_log.txt"
end

local function write_log(line)
    local path = get_log_path()
    local file, err = io.open(path, "a")
    if not file then
        vlc.msg.err("goo_logger_intf: cannot open " .. path .. ": " .. tostring(err))
        return false
    end
    file:write(line .. "\n")
    file:close()
    return true
end

local function get_current_uri()
    local input = vlc.object.input()
    if not input then
        return nil
    end
    
    local item = vlc.input.item()
    if not item then
        return nil
    end
    
    local uri = item:uri()
    if not uri or uri == "" then
        return nil
    end
    
    -- Decode URI if possible
    if vlc.strings and vlc.strings.decode_uri then
        local ok, decoded = pcall(vlc.strings.decode_uri, uri)
        if ok and decoded and decoded ~= "" then
            return decoded
        end
    end
    
    return uri
end

local function log_current_media()
    -- Check if something is playing
    local input = vlc.object.input()
    if not input then
        return
    end
    
    local uri = get_current_uri()
    if not uri then
        return
    end
    
    -- Only log if URI changed
    if uri == last_uri then
        return
    end
    
    -- Skip internal/special URIs
    if uri:match("^vlc://") or uri == "__activated__" then
        return
    end
    
    last_uri = uri
    local timestamp = os.date("!%Y-%m-%dT%H:%M:%SZ")
    local entry = timestamp .. "|" .. uri
    
    if write_log(entry) then
        vlc.msg.info("goo_logger_intf: logged " .. uri)
    end
end

-- Main script execution
vlc.msg.info("goo_logger_intf: script loaded, starting...")

-- Test log file access
local test_path = get_log_path()
vlc.msg.info("goo_logger_intf: log path = " .. test_path)

local test_file = io.open(test_path, "a")
if test_file then
    test_file:close()
    vlc.msg.info("goo_logger_intf: log file is writable")
    log_file_ok = true
else
    vlc.msg.err("goo_logger_intf: cannot write to log file!")
end

-- Main polling loop
vlc.msg.info("goo_logger_intf: entering main loop")
while true do
    if log_file_ok then
        local ok, err = pcall(log_current_media)
        if not ok then
            vlc.msg.err("goo_logger_intf: error in log_current_media: " .. tostring(err))
        end
    end
    vlc.misc.mwait(vlc.misc.mdate() + POLL_INTERVAL_US)
end
