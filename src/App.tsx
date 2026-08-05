/**
 * The window.
 *
 * The Craft Session panel **is** the app, and everything else here is support: which build is running,
 * whether an update is waiting, one real Win32 read, and the log tail. Those used to sit around the craft
 * panel as four full-width sections — a "This build" definition list above it, then Updates, then a
 * "Platform seam" panel with a *Read platform* button, then a raw `<pre>` of the log. Together they were
 * most of what [#9](https://github.com/Furizaa/poe-graft/issues/9)'s verdict called "walls of text and
 * debug stuff", so they are now behind one fold at the bottom.
 *
 * **Nothing was deleted.** On a machine with no dev environment this is the only diagnostic surface there
 * is, and the log is the only artifact that survives a relaunch. Two things are therefore still promoted
 * out of the fold, because they are not diagnostics — a waiting update, and a failed update — since both
 * need a decision from the human rather than merely being available to read.
 */
import { useCallback, useEffect, useRef, useState } from "react";
import { revealItemInDir } from "@tauri-apps/plugin-opener";
import { relaunch } from "@tauri-apps/plugin-process";
import { check, type Update } from "@tauri-apps/plugin-updater";
import {
  appendLog,
  getBuildInfo,
  getLogPath,
  getLogTail,
  getPlatformInfo,
  type BuildInfo,
  type PlatformInfo,
} from "./api";
import Craft from "./Craft";

/**
 * Where the update check has got to.
 *
 * `error` carries the raw string rather than a friendly message on purpose — this app is
 * debugged by reading it off a screen on a machine with no dev tools, so a paraphrase is worse
 * than useless.
 */
type UpdateStatus =
  | { kind: "idle" }
  | { kind: "checking" }
  | { kind: "current" }
  | { kind: "available"; update: Update }
  | { kind: "downloading"; version: string; received: number; total: number | null }
  | { kind: "installing"; version: string }
  | { kind: "error"; message: string };

const errorText = (error: unknown) =>
  error instanceof Error ? error.message : String(error);

const clockTime = () => new Date().toLocaleTimeString();

export default function App() {
  const [build, setBuild] = useState<BuildInfo | null>(null);
  const [logPath, setLogPath] = useState("");
  const [logLines, setLogLines] = useState<string[]>([]);
  const [platform, setPlatform] = useState<PlatformInfo | null>(null);
  const [platformError, setPlatformError] = useState<string | null>(null);
  const [status, setStatus] = useState<UpdateStatus>({ kind: "idle" });
  const [lastCheck, setLastCheck] = useState<string | null>(null);
  /** Guards the launch-time check against React's double-invoked effects in strict mode. */
  const checkedOnLaunch = useRef(false);

  const refreshLog = useCallback(async () => {
    setLogLines(await getLogTail());
  }, []);

  /** Write to the log file first, then pull it back — so what the panel shows is what persisted. */
  const record = useCallback(
    async (line: string) => {
      await appendLog(line);
      await refreshLog();
    },
    [refreshLog],
  );

  const checkForUpdates = useCallback(async () => {
    setStatus({ kind: "checking" });
    setLastCheck(clockTime());
    await record("updater: checking");
    try {
      const update = await check();
      if (!update) {
        setStatus({ kind: "current" });
        await record("updater: no update available");
        return;
      }
      setStatus({ kind: "available", update });
      await record(`updater: ${update.version} available (installed ${update.currentVersion})`);
    } catch (error) {
      const message = errorText(error);
      setStatus({ kind: "error", message });
      await record(`updater: check failed: ${message}`);
    }
  }, [record]);

  const install = useCallback(
    async (update: Update) => {
      try {
        setStatus({ kind: "downloading", version: update.version, received: 0, total: null });
        await record(`updater: downloading ${update.version}`);

        let received = 0;
        let total: number | null = null;
        await update.downloadAndInstall((event) => {
          switch (event.event) {
            case "Started":
              total = event.data.contentLength ?? null;
              break;
            case "Progress":
              received += event.data.chunkLength;
              setStatus({ kind: "downloading", version: update.version, received, total });
              break;
            case "Finished":
              setStatus({ kind: "installing", version: update.version });
              break;
          }
        });

        // Windows force-exits the app during the install step, so this line may never run.
        // That is exactly why it is written to a file rather than shown in the window.
        await record(`updater: installed ${update.version}, relaunching`);
        await relaunch();
      } catch (error) {
        const message = errorText(error);
        setStatus({ kind: "error", message });
        await record(`updater: install failed: ${message}`);
      }
    },
    [record],
  );

  const readPlatform = useCallback(async () => {
    try {
      setPlatform(await getPlatformInfo());
      setPlatformError(null);
    } catch (error) {
      setPlatform(null);
      setPlatformError(errorText(error));
    }
    await refreshLog();
  }, [refreshLog]);

  useEffect(() => {
    void (async () => {
      setBuild(await getBuildInfo());
      setLogPath(await getLogPath());
      await refreshLog();
      if (!checkedOnLaunch.current) {
        checkedOnLaunch.current = true;
        await checkForUpdates();
      }
    })();
  }, [checkForUpdates, refreshLog]);

  /** An update needs a decision; a check that failed needs to be seen. Everything else can wait. */
  const updateNeedsAttention =
    status.kind === "available" ||
    status.kind === "downloading" ||
    status.kind === "installing" ||
    status.kind === "error";

  return (
    <main>
      <header>
        <h1>poe-graft</h1>
        <span className="version">{build ? `v${build.version}` : "…"}</span>
        {build && (
          <span className="muted small">
            {build.runNumber ? `run #${build.runNumber}` : "local build"}
          </span>
        )}
      </header>

      {/* Promoted out of the fold: this is a decision, not a diagnostic. */}
      {updateNeedsAttention && (
        <div className={`notice${status.kind === "error" ? " bad" : ""}`}>
          {status.kind === "available" && (
            <>
              <span>
                Version <strong>{status.update.version}</strong> is available.
              </span>
              <button className="primary" onClick={() => install(status.update)}>
                Install and restart
              </button>
            </>
          )}
          {status.kind === "downloading" && (
            <span>
              Downloading {status.version}… {Math.round(status.received / 1024)} KB
              {status.total ? ` of ${Math.round(status.total / 1024)} KB` : ""}
            </span>
          )}
          {status.kind === "installing" && (
            <span>Installing {status.version}. The app will close and come back.</span>
          )}
          {status.kind === "error" && (
            <span className="error">Update check failed: {status.message}</span>
          )}
        </div>
      )}

      {/* The app. Everything above is a header and everything below is support. */}
      <Craft refreshLog={refreshLog} />

      <details className="app-fold">
        <summary>This build, updates and log</summary>

        <h3>Build</h3>
        {build ? (
          <dl>
            <dt>Version</dt>
            <dd className="mono">{build.version}</dd>
            <dt>Commit</dt>
            <dd className="mono">{build.commit}</dd>
            <dt>Actions run</dt>
            <dd>
              {build.runUrl ? (
                <a href={build.runUrl} target="_blank" rel="noreferrer">
                  #{build.runNumber}
                </a>
              ) : (
                <span className="muted">built locally</span>
              )}
            </dd>
            <dt>Platform</dt>
            <dd className="mono">{build.platform}</dd>
          </dl>
        ) : (
          <p className="muted">Loading…</p>
        )}

        <h3>Updates</h3>
        <div className="row">
          <button onClick={checkForUpdates} disabled={status.kind === "checking"}>
            Check for updates
          </button>
          {status.kind === "idle" && <span className="muted small">Not checked yet.</span>}
          {status.kind === "checking" && <span className="muted small">Checking…</span>}
          {status.kind === "current" && <span className="muted small">Up to date.</span>}
          {lastCheck && <span className="muted small">Last checked at {lastCheck}.</span>}
        </div>

        <h3>Platform seam</h3>
        <div className="row">
          <button onClick={readPlatform}>Read platform</button>
          {platform && (
            <span className="muted small mono">
              screen {platform.screenWidth}×{platform.screenHeight} · cursor {platform.cursorX},
              {platform.cursorY}
            </span>
          )}
          {!platform && !platformError && (
            <span className="muted small">
              One real Win32 read. On macOS this reports “not supported”, which is the stub doing its
              job.
            </span>
          )}
        </div>
        {platformError && <p className="error">{platformError}</p>}

        <h3>Log</h3>
        <div className="row">
          <button onClick={refreshLog}>Refresh</button>
          <button onClick={() => revealItemInDir(logPath)} disabled={!logPath}>
            Open folder
          </button>
          <span className="muted small mono">{logPath}</span>
        </div>
        <pre>{logLines.length ? logLines.join("\n") : "(empty)"}</pre>
      </details>
    </main>
  );
}
