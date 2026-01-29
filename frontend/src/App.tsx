import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Movie = {
  id: number;
  title: string;
  original_title?: string | null;
  overview?: string | null;
  release_date?: string | null;
  poster_path?: string | null;
};

type EnrichedEntry = {
  watched_at?: string | null;
  raw_title: string;
  cleaned_title: string;
  release_year?: number | null;
  movie?: Movie | null;
  tmdb_url?: string | null;
  poster_url?: string | null;
};

type AppSettings = {
  log_path?: string | null;
  cache_path?: string | null;
  tmdb_key_present?: boolean | null;
};

type SettingsInput = {
  log_path?: string | null;
  cache_path?: string | null;
  tmdb_api_key?: string | null;
};

type HistoryPayload = {
  entries: EnrichedEntry[];
  cache_warning?: string | null;
};

type GroupedEntry = {
  entry: EnrichedEntry;
  watch_dates: string[];
};

function formatDate(value?: string | null) {
  if (!value) {
    return "Date unknown";
  }
  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }
  return parsed.toLocaleDateString(undefined, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function releaseYear(value?: string | null) {
  if (!value) {
    return null;
  }
  const year = value.slice(0, 4);
  return /^\d{4}$/.test(year) ? year : null;
}

function normalizeSetting(value: string | null | undefined) {
  if (!value) {
    return null;
  }
  const trimmed = value.trim();
  return trimmed ? trimmed : null;
}

function resolveSetting(
  override: Partial<AppSettings> | undefined,
  key: keyof AppSettings,
  fallback: string
) {
  if (override && key in override) {
    const value = override[key];
    return typeof value === "string" ? value : "";
  }
  return fallback;
}

function formatWatchDates(values: string[]) {
  if (values.length === 0) {
    return { text: "Date unknown", full: "" };
  }
  const unique: string[] = [];
  const seen = new Set<string>();
  values.forEach(value => {
    if (!seen.has(value)) {
      seen.add(value);
      unique.push(value);
    }
  });
  const formatted = unique.map(formatDate);
  if (formatted.length <= 2) {
    const text = formatted.join(" · ");
    return { text, full: text };
  }
  const visible = formatted.slice(0, 2);
  const remaining = formatted.length - 2;
  return {
    text: `${visible.join(" · ")} · +${remaining} more`,
    full: formatted.join(" · "),
  };
}

export default function App() {
  const [entries, setEntries] = useState<EnrichedEntry[]>([]);
  const [warning, setWarning] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<"loading" | "idle" | "error">("loading");
  const [logPath, setLogPath] = useState("");
  const [cachePath, setCachePath] = useState("");
  const [tmdbApiKey, setTmdbApiKey] = useState("");
  const [tmdbKeyPresent, setTmdbKeyPresent] = useState(false);
  const [showSettings, setShowSettings] = useState(false);

  const buildSettingsPayload = (overrides?: Partial<AppSettings>): SettingsInput => ({
    log_path: normalizeSetting(resolveSetting(overrides, "log_path", logPath)),
    cache_path: normalizeSetting(resolveSetting(overrides, "cache_path", cachePath)),
    tmdb_api_key: normalizeSetting(tmdbApiKey),
  });

  const saveSettings = async () => {
    const payload = buildSettingsPayload();
    await invoke("save_settings", { settings: payload });
  };

  const loadHistory = async (overrides?: Partial<AppSettings>) => {
    setStatus("loading");
    setError(null);
    const settingsPayload = buildSettingsPayload(overrides);
    try {
      const payload = await invoke<HistoryPayload>("load_history", {
        logPath: settingsPayload.log_path,
        cachePath: settingsPayload.cache_path,
        tmdbApiKey: settingsPayload.tmdb_api_key,
      });
      setEntries(payload.entries ?? []);
      setWarning(payload.cache_warning ?? null);
      setStatus("idle");
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
      setStatus("error");
    }
  };

  useEffect(() => {
    const init = async () => {
      try {
        const settings = await invoke<AppSettings>("load_settings");
        setLogPath(settings.log_path ?? "");
        setCachePath(settings.cache_path ?? "");
        setTmdbKeyPresent(Boolean(settings.tmdb_key_present));
        setTmdbApiKey("");
        await loadHistory(settings);
      } catch {
        await loadHistory();
      }
    };
    void init();
  }, []);

  const items = useMemo(() => {
    const byKey = new Map<string, GroupedEntry>();
    const order: string[] = [];
    const reversed = entries.slice().reverse();
    reversed.forEach(entry => {
      const year = entry.release_year ?? releaseYear(entry.movie?.release_date ?? null);
      const key = `${entry.cleaned_title.toLowerCase()}${year ? `|${year}` : ""}`;
      const existing = byKey.get(key);
      if (existing) {
        const watched = entry.watched_at;
        if (watched && !existing.watch_dates.includes(watched)) {
          existing.watch_dates.push(watched);
        }
        return;
      }
      const watched = entry.watched_at;
      byKey.set(key, {
        entry,
        watch_dates: watched ? [watched] : [],
      });
      order.push(key);
    });
    return order.map(key => byKey.get(key)).filter(Boolean) as GroupedEntry[];
  }, [entries]);

  return (
    <div className="page">
      <header className="header">
        <div className="title-block">
          <h1>GOO</h1>
          <p className="subtitle">Watch history</p>
        </div>
        <div className="header-actions">
          <button
            className="icon-button"
            onClick={() => setShowSettings(true)}
            title="Settings"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="currentColor">
              <circle cx="12" cy="5" r="2" />
              <circle cx="12" cy="12" r="2" />
              <circle cx="12" cy="19" r="2" />
            </svg>
          </button>
          <button
            className="icon-button"
            onClick={loadHistory}
            disabled={status === "loading"}
            title="Refresh"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.2" />
            </svg>
          </button>
        </div>
      </header>

      {showSettings && (
        <div className="modal-overlay" onClick={() => setShowSettings(false)}>
          <div className="modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <h2>Settings</h2>
              <button className="close-button" onClick={() => setShowSettings(false)}>
                ✕
              </button>
            </div>
            <div className="modal-body">
              <label className="field">
                <span>Log path</span>
                <input
                  value={logPath}
                  onChange={(event) => setLogPath(event.target.value)}
                  placeholder="Auto-detect or set GOO_LOG_PATH"
                />
              </label>
              <label className="field">
                <span>Cache path</span>
                <input
                  value={cachePath}
                  onChange={(event) => setCachePath(event.target.value)}
                  placeholder="Optional .goo_cache.json"
                />
              </label>
              <label className="field">
                <span>TMDB API Key</span>
                <input
                  type="password"
                  value={tmdbApiKey}
                  onChange={(event) => setTmdbApiKey(event.target.value)}
                  placeholder={tmdbKeyPresent ? "Saved in Windows Credential Manager" : "Optional (or set TMDB_API_KEY env)"}
                />
              </label>
            </div>
            <div className="modal-footer">
              {tmdbKeyPresent && (
                <button
                  className="secondary"
                  onClick={() => {
                    invoke("clear_tmdb_key")
                      .then(() => {
                        setTmdbKeyPresent(false);
                        setTmdbApiKey("");
                      })
                      .catch(err => {
                        const message = err instanceof Error ? err.message : String(err);
                        setError(message);
                        setStatus("error");
                      });
                  }}
                >
                  Clear Key
                </button>
              )}
              <button className="secondary" onClick={() => setShowSettings(false)}>
                Cancel
              </button>
              <button
                className="primary"
                onClick={() => {
                  setShowSettings(false);
                  saveSettings()
                    .then(() => {
                      if (tmdbApiKey.trim()) {
                        setTmdbKeyPresent(true);
                        setTmdbApiKey("");
                      }
                      return loadHistory();
                    })
                    .catch(err => {
                      const message = err instanceof Error ? err.message : String(err);
                      setError(message);
                      setStatus("error");
                    });
                }}
              >
                Save & Refresh
              </button>
            </div>
          </div>
        </div>
      )}

      {warning && <div className="banner warning">Cache: {warning}</div>}
      {error && (
        <div className="banner error">
          {error}
          <span className="hint">
            If this mentions TMDB, set a key in Settings or via TMDB_API_KEY.
          </span>
        </div>
      )}

      {status === "loading" && (
        <div className="status">Loading your latest plays.</div>
      )}

      {status !== "loading" && items.length === 0 && (
        <div className="status empty">No history yet. Play something in VLC.</div>
      )}

      <section className="grid">
        {items.map((item, index) => {
          const entry = item.entry;
          const title = entry.movie?.title ?? entry.cleaned_title;
          const year = entry.release_year ?? releaseYear(entry.movie?.release_date ?? null);
          const dateInfo = formatWatchDates(item.watch_dates);
          const dateTitle = dateInfo.full !== dateInfo.text ? dateInfo.full : undefined;
          const tmdbLink =
            entry.tmdb_url ??
            `https://www.themoviedb.org/search?query=${encodeURIComponent(
              entry.cleaned_title
            )}`;
          const poster = entry.poster_url ?? null;

          return (
            <article className="card" key={`${entry.raw_title}-${index}`}>
              <a className="poster" href={tmdbLink} target="_blank" rel="noreferrer">
                {poster ? (
                  <img src={poster} alt={`${title} poster`} loading="lazy" />
                ) : (
                  <div className="poster-fallback">
                    <span>{title}</span>
                    <em>No poster</em>
                  </div>
                )}
              </a>
              <div className="card-body">
                <div className="title-row">
                  <h3 title={title}>{title}</h3>
                  {year && <span className="badge">{year}</span>}
                </div>
                <div className="meta">
                  <span className="meta-item" title={dateTitle}>
                    {dateInfo.text}
                  </span>
                  <a className="tmdb-link" href={tmdbLink} target="_blank" rel="noreferrer">
                    TMDB
                  </a>
                </div>
              </div>
            </article>
          );
        })}
      </section>
    </div>
  );
}
