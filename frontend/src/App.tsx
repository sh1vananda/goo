import { useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-shell";

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

type Recommendation = {
  title: string;
  year: number;
  director: string;
  genres: string[];
};

type EnrichedRecommendation = Recommendation & {
  poster_url?: string | null;
  tmdb_url?: string | null;
};

type AppSettings = {
  log_path?: string | null;
  cache_path?: string | null;
  tmdb_key_present?: boolean | null;
  nim_key_present?: boolean | null;
};

type SettingsInput = {
  log_path?: string | null;
  cache_path?: string | null;
  tmdb_api_key?: string | null;
  nim_api_key?: string | null;
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
  // Only display the most recent date
  const latest = values[values.length - 1];
  const text = formatDate(latest);
  return { text, full: text };
}

function isTauriRuntime() {
  return typeof window !== "undefined" && Boolean((window as any).__TAURI__);
}

async function openExternalLink(url: string) {
  if (isTauriRuntime()) {
    await open(url);
    return;
  }
  window.open(url, "_blank", "noopener,noreferrer");
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
  const [nimApiKey, setNimApiKey] = useState("");
  const [nimKeyPresent, setNimKeyPresent] = useState(false);
  const [showSettings, setShowSettings] = useState(false);
  const [showRecsModal, setShowRecsModal] = useState(false);
  const [busyAction, setBusyAction] = useState<"delete-log" | "delete-entry" | "get-recs" | null>(null);
  const [recommendations, setRecommendations] = useState<EnrichedRecommendation[]>(() => {
    const saved = localStorage.getItem("goo_recommendations");
    try {
      const parsed = saved ? JSON.parse(saved) : [];
      return Array.isArray(parsed) ? parsed : [];
    } catch {
      return [];
    }
  });
  const [manualExclusions, setManualExclusions] = useState<string[]>(() => {
    const saved = localStorage.getItem("goo_manual_exclusions");
    const parsed = saved ? JSON.parse(saved) : [];
    return Array.isArray(parsed) ? parsed : [];
  });
  const [rawExclusionText, setRawExclusionText] = useState(manualExclusions.join(", "));

  // Sync raw text when manual exclusions change from external actions (like marking as watched)
  useEffect(() => {
    const joined = manualExclusions.join(", ");
    // Only update if the parsed content is different to avoid cursor jumps
    const currentTitles = rawExclusionText.split(",").map(t => t.trim()).filter(t => t !== "");
    if (JSON.stringify(currentTitles) !== JSON.stringify(manualExclusions)) {
      setRawExclusionText(joined);
    }
  }, [manualExclusions]);


  const buildSettingsPayload = (overrides?: Partial<AppSettings>): SettingsInput => ({
    log_path: normalizeSetting(resolveSetting(overrides, "log_path", logPath)),
    cache_path: normalizeSetting(resolveSetting(overrides, "cache_path", cachePath)),
    tmdb_api_key: normalizeSetting(tmdbApiKey),
    nim_api_key: normalizeSetting(nimApiKey),
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

  const getRecommendations = async () => {
    setBusyAction("get-recs");
    setError(null);
    try {
      // Merge log titles and manual exclusions for AI
      const logTitles = Array.from(new Set(entries.map(e => e.cleaned_title)));
      const exclusionList = Array.from(new Set([...logTitles, ...manualExclusions]));

      const recs = await invoke<EnrichedRecommendation[]>("get_recommendations", {
        exclusionList,
        nimApiKey: normalizeSetting(nimApiKey) || null,
        tmdbApiKey: normalizeSetting(tmdbApiKey) || null,
      });

      setRecommendations(recs);
      localStorage.setItem("goo_recommendations", JSON.stringify(recs));
    } catch (err) {
      const message = err instanceof Error ? err.message : String(err);
      setError(message);
    } finally {
      setBusyAction(null);
    }
  };

  const addExclusion = (title: string) => {
    const updated = Array.from(new Set([...manualExclusions, title]));
    setManualExclusions(updated);
    localStorage.setItem("goo_manual_exclusions", JSON.stringify(updated));
  };

  useEffect(() => {
    const init = async () => {
      try {
        const settings = await invoke<AppSettings>("load_settings");
        setLogPath(settings.log_path ?? "");
        setCachePath(settings.cache_path ?? "");
        setTmdbKeyPresent(Boolean(settings.tmdb_key_present));
        setNimKeyPresent(Boolean(settings.nim_key_present));
        setTmdbApiKey("");
        setNimApiKey("");
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
        // Rewatch: replace the date with the more recent one
        const watched = entry.watched_at;
        if (watched) {
          existing.watch_dates = [watched];
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
            onClick={() => setShowRecsModal(true)}
            title="Curator Recommendations"
          >
            <svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round">
              <path d="M12 2l3.09 6.26L22 9.27l-5 4.87 1.18 6.88L12 17.77l-6.18 3.25L7 14.14 2 9.27l6.91-1.01L12 2z" />
            </svg>
          </button>
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
            onClick={() => loadHistory()}
            disabled={status === "loading"}
            title="Refresh History"
          >
            <svg
              width="18"
              height="18"
              viewBox="0 0 24 24"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              className={status === "loading" ? "spin" : ""}
            >
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
              <label className="field">
                <span>NVIDIA NIM API Key</span>
                <input
                  type="password"
                  value={nimApiKey}
                  onChange={(event) => setNimApiKey(event.target.value)}
                  placeholder={nimKeyPresent ? "Saved in Windows Credential Manager" : "Required for recommendations"}
                />
              </label>
            </div>
            <div className="modal-footer">
              {(tmdbKeyPresent || nimKeyPresent) && (
                <button
                  className="secondary"
                  onClick={() => {
                    invoke("clear_keys")
                      .then(() => {
                        setTmdbKeyPresent(false);
                        setNimKeyPresent(false);
                        setTmdbApiKey("");
                        setNimApiKey("");
                      })
                      .catch(err => {
                        const message = err instanceof Error ? err.message : String(err);
                        setError(message);
                        setStatus("error");
                      });
                  }}
                >
                  Clear Keys
                </button>
              )}
              <button
                className="secondary"
                onClick={async () => {
                  const ok = window.confirm(
                    "Clear the entire watch log? VLC will recreate it on next play."
                  );
                  if (!ok) return;
                  setBusyAction("delete-log");
                  try {
                    await invoke("delete_log", {
                      logPath: logPath.trim() ? logPath.trim() : null,
                    });
                    await loadHistory();
                  } catch (err) {
                    const message = err instanceof Error ? err.message : String(err);
                    setError(message);
                    setStatus("error");
                  } finally {
                    setBusyAction(null);
                  }
                }}
                disabled={busyAction !== null}
              >
                Clear Log
              </button>
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
                      if (nimApiKey.trim()) {
                        setNimKeyPresent(true);
                        setNimApiKey("");
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

      {showRecsModal && (
        <div className="modal-overlay" onClick={() => setShowRecsModal(false)}>
          <div className="modal recs-modal" onClick={(e) => e.stopPropagation()}>
            <div className="modal-header">
              <div className="modal-title-block">
                <h2>Recommendations</h2>
                <div className="reco-status">
                  {busyAction === "get-recs" ? (
                    <span className="curating">Fetching new insights...</span>
                  ) : recommendations.length === 0 ? (
                    <span>Add to your history for new insights.</span>
                  ) : null}
                </div>
              </div>
              <div className="modal-header-actions">
                <button
                  className="icon-button"
                  onClick={getRecommendations}
                  disabled={busyAction === "get-recs"}
                  title="Refresh Recommendations"
                >
                  <svg
                    width="16"
                    height="16"
                    viewBox="0 0 24 24"
                    fill="none"
                    stroke="currentColor"
                    strokeWidth="2"
                    className={busyAction === "get-recs" ? "spin" : ""}
                  >
                    <path d="M21.5 2v6h-6M2.5 22v-6h6M2 11.5a10 10 0 0 1 18.8-4.3M22 12.5a10 10 0 0 1-18.8 4.2" />
                  </svg>
                </button>
                <button className="close-button" onClick={() => setShowRecsModal(false)}>
                  ✕
                </button>
              </div>
            </div>

            <div className="recs-layout">
              <aside className="recs-sidebar">
                <div className="sidebar-section">
                  <h3>Exclusion List</h3>
                  <textarea
                    className="exclusion-textarea"
                    value={rawExclusionText}
                    onChange={(e) => {
                      const text = e.target.value;
                      setRawExclusionText(text);
                      const titles = text.split(",")
                        .map(t => t.trim())
                        .filter(t => t !== "");
                      setManualExclusions(titles);
                      localStorage.setItem("goo_manual_exclusions", JSON.stringify(titles));
                    }}
                    placeholder="Titles separated by commas..."
                  />
                  <span className="hint">Comma separated list of titles the AI will never suggest, but will use to fuel better fits.</span>
                </div>
              </aside>

              <div className="recs-body">
                {recommendations.length > 0 ? (
                  <div className="recs-grid">
                    {recommendations.map((rec, index) => {
                      const title = rec.title;
                      const tmdbLink = rec.tmdb_url || `https://www.themoviedb.org/search?query=${encodeURIComponent(title)}`;
                      return (
                        <article className="card reco-card" key={`${title}-${index}`}>
                          <div
                            className="poster"
                            onClick={(event) => {
                              event.preventDefault();
                              if (isTauriRuntime()) {
                                openExternalLink(tmdbLink).catch(() => { });
                              } else {
                                window.open(tmdbLink, "_blank", "noopener,noreferrer");
                              }
                            }}
                          >
                            {rec.poster_url ? (
                              <img src={rec.poster_url} alt={`${title} poster`} />
                            ) : (
                              <div className="poster-fallback">
                                <span>{title}</span>
                              </div>
                            )}
                            <div className="poster-overlay">
                              <button
                                className="mark-watched-icon"
                                title="Mark as watched and remove"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  addExclusion(title);
                                  setRecommendations(prev => prev.filter((_, i) => i !== index));
                                }}
                              >
                                <svg width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" strokeLinejoin="round">
                                  <polyline points="20 6 9 17 4 12" />
                                </svg>
                              </button>
                            </div>
                          </div>
                          <div className="card-body">
                            <div className="title-row">
                              <h3 title={title}>{title}</h3>
                              <span className="badge">{rec.year}</span>
                            </div>
                            <div className="director">{rec.director}</div>
                            <div className="reco-genres">
                              {rec.genres?.map((g, i) => (
                                <span key={i} className="genre-tag">{g}</span>
                              ))}
                            </div>
                          </div>
                        </article>
                      );
                    })}
                  </div>
                ) : (
                  <div className="status empty">
                    {busyAction === "get-recs" ? "Selecting titles..." : "Consult the curator for elite suggestions."}
                  </div>
                )}
              </div>
            </div>
          </div>
        </div>
      )}

      {status === "idle" && items.length === 0 && (
        <div className="status empty">No history yet. Play something in VLC.</div>
      )}

      <section className="grid">
        {items.map((item, index) => {
          const entry = item.entry;
          const title = entry.movie?.title ?? entry.cleaned_title;
          const year = entry.release_year ?? releaseYear(entry.movie?.release_date ?? null);
          const dateInfo = formatWatchDates(item.watch_dates);
          const dateTitle = dateInfo.full !== dateInfo.text ? dateInfo.full : undefined;
          const dateTooltip = dateTitle ?? undefined;
          const tmdbLink =
            entry.tmdb_url ??
            `https://www.themoviedb.org/search?query=${encodeURIComponent(
              entry.cleaned_title
            )}`;
          const poster = entry.poster_url ?? null;

          return (
            <article className="card" key={`${entry.raw_title}-${index}`}>
              <a
                className="poster"
                href={tmdbLink}
                target="_blank"
                rel="noreferrer"
                onClick={(event) => {
                  if (isTauriRuntime()) {
                    event.preventDefault();
                    openExternalLink(tmdbLink).catch(() => { });
                  }
                }}
              >
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
                  <span
                    className={dateTooltip ? "meta-item tooltip" : "meta-item"}
                    data-tooltip={dateTooltip}
                  >
                    {dateInfo.text}
                  </span>
                  <div className="meta-actions">
                    <a
                      className="tmdb-link"
                      href={tmdbLink}
                      target="_blank"
                      rel="noreferrer"
                      onClick={(event) => {
                        if (isTauriRuntime()) {
                          event.preventDefault();
                          openExternalLink(tmdbLink).catch(() => { });
                        }
                      }}
                    >
                      TMDB
                    </a>
                    <button
                      className="meta-button"
                      onClick={async () => {
                        const ok = window.confirm(
                          "Remove this title from your history?"
                        );
                        if (!ok) return;
                        addExclusion(entry.cleaned_title);
                        setBusyAction("delete-entry");
                        try {
                          await invoke("delete_entry", {
                            logPath: logPath.trim() ? logPath.trim() : null,
                            cleanedTitle: entry.cleaned_title,
                            releaseYear: entry.release_year ?? null,
                          });
                          await loadHistory();
                        } catch (err) {
                          const message = err instanceof Error ? err.message : String(err);
                          setError(message);
                        } finally {
                          setBusyAction(null);
                        }
                      }}
                      disabled={busyAction !== null}
                      title="Remove and mark as watched"
                    >
                      <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                        <path d="M3 6h18" />
                        <path d="M8 6V4h8v2" />
                        <path d="M6 6l1 14h10l1-14" />
                      </svg>
                    </button>
                  </div>
                </div>
              </div>
            </article>
          );
        })}
      </section>
    </div>
  );
}
