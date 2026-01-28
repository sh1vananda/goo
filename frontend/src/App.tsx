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
  movie?: Movie | null;
  tmdb_url?: string | null;
  poster_url?: string | null;
};

type HistoryPayload = {
  entries: EnrichedEntry[];
  cache_warning?: string | null;
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

function truncate(text?: string | null, max = 170) {
  if (!text) {
    return "No synopsis available.";
  }
  if (text.length <= max) {
    return text;
  }
  return `${text.slice(0, max).trim()}...`;
}

export default function App() {
  const [entries, setEntries] = useState<EnrichedEntry[]>([]);
  const [warning, setWarning] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [status, setStatus] = useState<"loading" | "idle" | "error">("loading");
  const [logPath, setLogPath] = useState(() => localStorage.getItem("goo_log_path") || "");
  const [cachePath, setCachePath] = useState(() => localStorage.getItem("goo_cache_path") || "");
  const [tmdbApiKey, setTmdbApiKey] = useState(() => localStorage.getItem("goo_tmdb_key") || "");
  const [showSettings, setShowSettings] = useState(false);

  const saveSettings = () => {
    localStorage.setItem("goo_log_path", logPath);
    localStorage.setItem("goo_cache_path", cachePath);
    localStorage.setItem("goo_tmdb_key", tmdbApiKey);
  };

  const loadHistory = async () => {
    setStatus("loading");
    setError(null);
    saveSettings();
    try {
      const payload = await invoke<HistoryPayload>("load_history", {
        logPath: logPath.trim() ? logPath.trim() : null,
        cachePath: cachePath.trim() ? cachePath.trim() : null,
        tmdbApiKey: tmdbApiKey.trim() ? tmdbApiKey.trim() : null,
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
    loadHistory();
  }, []);

  const items = useMemo(() => entries.slice().reverse(), [entries]);

  return (
    <div className="page">
      <header className="header">
        <div className="title-block">
          <p className="eyebrow">Goo</p>
          <h1>Quiet watch history</h1>
          <p className="subtitle">
            A lightweight gallery of what VLC has been playing.
          </p>
        </div>
        <div className="header-actions">
          <button
            className="icon-button"
            onClick={() => setShowSettings(true)}
            title="Settings"
          >
            ⚙️
          </button>
          <button
            className="primary"
            onClick={loadHistory}
            disabled={status === "loading"}
          >
            {status === "loading" ? "Loading..." : "Refresh"}
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
                  placeholder="Optional (or set TMDB_API_KEY env)"
                />
              </label>
            </div>
            <div className="modal-footer">
              <button className="secondary" onClick={() => setShowSettings(false)}>
                Cancel
              </button>
              <button
                className="primary"
                onClick={() => {
                  setShowSettings(false);
                  loadHistory();
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
            If this mentions TMDB, set TMDB_API_KEY for the backend.
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
        {items.map((entry, index) => {
          const title = entry.movie?.title ?? entry.cleaned_title;
          const year = releaseYear(entry.movie?.release_date ?? null);
          const overview = truncate(entry.movie?.overview ?? null);
          const watched = formatDate(entry.watched_at ?? null);
          const tmdbLink =
            entry.tmdb_url ??
            `https://www.themoviedb.org/search?query=${encodeURIComponent(
              entry.cleaned_title
            )}`;
          const poster = entry.poster_url ?? null;
          const delay = `${Math.min(index, 20) * 40}ms`;

          return (
            <article className="card" style={{ animationDelay: delay }} key={`${entry.raw_title}-${index}`}>
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
                  <h3>{title}</h3>
                  {year && <span className="badge">{year}</span>}
                </div>
                <p className="overview">{overview}</p>
                <div className="meta">
                  <span className="meta-item">Watched {watched}</span>
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
