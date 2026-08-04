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

      <section>
        <h2>This build</h2>
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
      </section>

      <section>
        <h2>Updates</h2>
        <div className="row">
          <button onClick={checkForUpdates} disabled={status.kind === "checking"}>
            Check for updates
          </button>
          {status.kind === "available" && (
            <button className="primary" onClick={() => install(status.update)}>
              Install {status.update.version} and restart
            </button>
          )}
        </div>
        <p className="status">
          {status.kind === "idle" && <span className="muted">Not checked yet.</span>}
          {status.kind === "checking" && "Checking…"}
          {status.kind === "current" && "Up to date."}
          {status.kind === "available" && `Version ${status.update.version} is available.`}
          {status.kind === "downloading" &&
            `Downloading ${status.version}… ${Math.round(status.received / 1024)} KB${
              status.total ? ` of ${Math.round(status.total / 1024)} KB` : ""
            }`}
          {status.kind === "installing" &&
            `Installing ${status.version}. The app will close and come back.`}
          {status.kind === "error" && <span className="error">{status.message}</span>}
        </p>
        {lastCheck && <p className="muted small">Last checked at {lastCheck}.</p>}
      </section>

      <section>
        <h2>Platform seam</h2>
        <div className="row">
          <button onClick={readPlatform}>Read platform</button>
        </div>
        {platform && (
          <p className="mono">
            screen {platform.screenWidth}×{platform.screenHeight} · cursor {platform.cursorX},
            {platform.cursorY}
          </p>
        )}
        {platformError && <p className="error">{platformError}</p>}
        {!platform && !platformError && (
          <p className="muted small">
            One real Win32 read. On macOS this reports “not supported”, which is the stub doing
            its job.
          </p>
        )}
      </section>

      <section>
        <h2>Log</h2>
        <div className="row">
          <button onClick={refreshLog}>Refresh</button>
          <button onClick={() => revealItemInDir(logPath)} disabled={!logPath}>
            Open folder
          </button>
        </div>
        <p className="muted small mono">{logPath}</p>
        <pre>{logLines.length ? logLines.join("\n") : "(empty)"}</pre>
      </section>
    </main>
  );
}
