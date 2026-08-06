# Windows builds and auto-update via GitHub Actions

Research for [#5](https://github.com/Furizaa/poe-graft/issues/5). Dated 2026-08-04.

**Framing.** There is no dev environment on the Windows gaming PC, so `merge → Actions → the installed app updates itself` is the project's *inner loop*, not release plumbing. Everything below is judged on round-trip latency and on how often the loop breaks in a way you cannot debug from the Mac.

**One fact that shapes every answer:** `Furizaa/poe-graft` is **private** (`gh repo view --json isPrivate` → `true`). That has two consequences that are easy to miss and both were verified empirically, not assumed — see [Private repositories](#private-repositories-the-single-biggest-gotcha).

---

## Verdict

| | **Tauri v2** | **Electron** | **.NET + Velopack** |
|---|---|---|---|
| **Pipeline simplicity** | Good — one action (`tauri-action`) does build + release + `latest.json`. **But** the updater does not work against a private repo without a workaround. | **Best** — `electron-builder --publish always`, private repos supported natively by design. | Good — four `vpk` CLI lines, private repos supported natively. |
| **Cold CI build** (windows-latest, private repo = 2 vCPU / 8 GB) | **10–18 min** (Rust release build of ~400–500 crates) | 4–7 min | **3–5 min** |
| **Warm CI build** (cache hit) | 4–6 min | 4–6 min | 3–5 min |
| **Artifact size / download** | **5–15 MB** installer — near-instant on the target | 80–250 MB, but `.blockmap` differential download makes the *second* update small | 30–80 MB full, delta packages make updates small |
| **Updater maturity** | First-party, Tauri-team maintained, mandatory signature. Solid. | **Most mature** on Windows. Years of production use, richest event/API surface. | Mature (Rust-based rewrite of Squirrel, actively maintained). Squirrel.Windows is dead, ClickOnce is legacy — Velopack is the answer here. |
| **Signing the updater needs** | **Mandatory** minisign keypair — free, self-generated, no CA. | Only a SHA-512 in `latest.yml` (auto-generated). Authenticode check **auto-skipped** when unsigned. | SHA verification only. Authenticode optional. |
| **Authenticode / SmartScreen** | Optional | Optional | Optional |
| **Private-repo updater** | ✗ Needs a workaround (public releases repo, proxy, or make repo public) | ✓ Native (`private: true` + token on the client) | ✓ Native (`GithubSource(url, token)`) |
| **Fits the Mac dev loop** | ✓ Rust + Node already installed | ✓ Node already installed | ✗ No `dotnet` on the Mac; WPF/WinUI cannot run on macOS at all |

### Recommendation

**Tauri v2, and make the repository public** (or, if the source must stay private, publish artifacts to a separate *public* `poe-graft-releases` repo — `tauri-action` has `owner`/`repo` inputs for exactly this).

Going public buys three things at once:
1. The Tauri updater works with **zero auth** — `https://github.com/Furizaa/poe-graft/releases/latest/download/latest.json` is just a public URL. No embedded token, no proxy.
2. `windows-latest` becomes **4 vCPU / 16 GB** instead of 2 vCPU / 8 GB, roughly halving the cold Rust build.
3. Actions minutes become **free and unlimited**. On a private repo, Windows minutes bill at ~1.7× Linux ($0.010/min vs $0.006/min), and a 6-minute build costs 6 minutes against a 2,000–3,000/month allowance.

For a single-user PoE crafting helper there is no reason for the repo to be private. This is the single highest-leverage decision in this ticket.

**Expected round trip, Tauri + public repo, warm cache:**

| Stage | Time |
|---|---|
| Merge → workflow starts | 5–20 s |
| checkout + node + rust-cache restore + `pnpm i` | 60–90 s |
| `tauri build` (your crate recompiles + links; deps cached) | 120–210 s |
| NSIS bundle + sign + upload assets + `latest.json` | 30–60 s |
| Click "Check for updates" in the app on the gaming PC | instant |
| Download 5–15 MB + passive NSIS install + relaunch | 10–20 s |
| **Total** | **≈ 5–7 min** |

Cold (first run, or any `Cargo.lock` change): **12–20 min.** That is the loop's worst case and it is the main argument against Tauri.

**Fallback:** if the private-repo dance or cold Rust builds prove intolerable, **Electron** is the low-friction alternative — the pipeline is genuinely simpler, private repos work out of the box, and warm builds are comparable. You pay in artifact size and RAM footprint next to a running PoE client.

**Do not pick .NET.** Not because Velopack is bad — it is the best of the three at pure CI speed and delta updates — but because it breaks the *other* half of the loop. There is no `dotnet` on the Mac (per the map's environment facts), and a WPF/WinUI UI cannot be run or iterated on macOS at all. Only Avalonia keeps the Mac usable, which means adopting both a new toolchain and a less-travelled UI stack to gain ~2 minutes of CI. Bad trade.

---

## Detailed findings

### 1. Can a Mac-authored repo build Windows entirely in CI?

Yes, for all three, with no local Windows toolchain. All three run on `windows-latest`, which currently maps to **Windows Server 2025**. The runner image ships MSVC build tools, the Windows SDK, .NET SDKs, and Node; Rust is added by `dtolnay/rust-toolchain`.

Runner hardware ([GitHub docs](https://docs.github.com/en/actions/reference/runners/github-hosted-runners)):

| | vCPU | RAM | SSD |
|---|---|---|---|
| `windows-latest`, **public** repo | 4 | 16 GB | 14 GB |
| `windows-latest`, **private** repo | 2 | 8 GB | 14 GB |

> "Use of the standard GitHub-hosted runners is free and unlimited on public repositories."

Cross-compiling Windows from the Mac is not on the table for any of them (NSIS/WiX/MSVC linkage, `dotnet publish -r win-x64` needs Windows for the Velopack bootstrapper). CI is the only path — which is exactly the premise of the ticket.

**Permissions.** Every one of these workflows needs:

```yaml
permissions:
  contents: write
```

The auto-issued `secrets.GITHUB_TOKEN` is sufficient to create releases and upload assets in the *same* repo. Publishing to a *different* repo (the public-releases-repo pattern) needs a PAT with `contents: write` on that repo.

### 2. Build times and cache strategy

**Tauri.** The dominant cost is the Rust release build. A `create-tauri-app` baseline pulls ~400–500 crates; on a 2-vCPU Windows runner a cold `--release` build lands in the **8–15 min** range, plus ~2 min of checkout/toolchain/frontend and ~1 min of NSIS bundling. Community reports put a *cached* Tauri Windows build at [4–8 minutes](https://dev.to/tomtomdu73/ship-your-tauri-v2-app-like-a-pro-github-actions-and-release-automation-part-22-2ef7) and an uncached one at 5–15 min; the official pipeline docs note that an uncached ARM build of a fresh project "needs ~1 hour" on emulated runners, which calibrates how heavy cold Rust is.

Cache strategy — all three of these matter:

- `swatinem/rust-cache@v2` with `workspaces: './src-tauri -> target'`. This is in the [official Tauri pipeline example](https://v2.tauri.app/distribute/pipelines/github/). It caches the registry, the git db, and `target/` for **dependencies only** — it deliberately evicts your own workspace crates, so `poe-graft`'s own code recompiles and re-links every run. That floor is ~1.5–3.5 min on 2 vCPU and cannot be cached away.
- `actions/setup-node` with `cache: 'pnpm'` (or npm) for the frontend.
- **Cache key sensitivity is the real trap.** `rust-cache` keys on `Cargo.lock`. Add or bump one dependency and your next merge is a cold 12–18 min build. Batch dependency changes; don't interleave them with feature work you want to test on Windows.
- Actions cache is capped at **10 GB per repository**. A Tauri `target/` cache is a few hundred MB compressed, so this is fine, but don't add unrelated large caches.

**Electron.** No compilation unless you have native modules. `npm ci` 30–60 s, Electron binary download ~100 MB (cache `~\AppData\Local\electron` — the [official Actions guide](https://github.com/electron-userland/electron-builder/blob/master/website/docs/features/github-actions.md) does this), `electron-builder --win` NSIS packaging 2–4 min. **Total 4–7 min, stable, no cold/warm cliff.** Caveat for *this* project: a global low-level keyboard/mouse hook means a native module (`uiohook-napi`, `node-global-key-listener`, or similar). If prebuilds exist for your Electron ABI you stay fast; if `node-gyp` has to compile, add 3–5 min and a new class of CI failure. Use `npm ci`, not `npm install`, so native modules rebuild against the right Electron version.

**.NET + Velopack.** `actions/setup-dotnet` 20–40 s, `dotnet publish -r win-x64 --self-contained` 1–2 min, `dotnet tool install -g vpk` ~30 s, `vpk pack` 30–60 s. **Total 3–5 min** and no cold cliff. Cache `~/.nuget/packages`. Fastest CI of the three.

### 3. Auto-update mechanism, end to end

#### Tauri v2 — `@tauri-apps/plugin-updater`

**Setup.** Generate a keypair once, on the Mac:

```bash
pnpm tauri signer generate -w ~/.tauri/poe-graft.key
```

This is the `tauri signer` CLI ("Generate signing keys for Tauri updater or sign files", [CLI reference](https://v2.tauri.app/reference/cli)). It emits a private key (→ GitHub secret) and a public key (→ committed into `tauri.conf.json`).

```json
{
  "bundle": {
    "createUpdaterArtifacts": true
  },
  "plugins": {
    "updater": {
      "pubkey": "CONTENT FROM PUBLICKEY.PEM",
      "endpoints": [
        "https://github.com/Furizaa/poe-graft/releases/latest/download/latest.json"
      ],
      "windows": { "installMode": "passive" }
    }
  }
}
```

- `createUpdaterArtifacts: true` is **required** — without it the build emits no `.sig` files and `tauri-action` has nothing to build `latest.json` from.
- The endpoint list supports `{{target}}`, `{{arch}}`, `{{current_version}}` substitution (done by string replace in [`updater.rs`](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/updater.rs)). TLS is enforced in production.

**Manifest.** `latest.json` (the static-file form):

```json
{
  "version": "",
  "notes": "",
  "pub_date": "",
  "platforms": {
    "windows-x86_64": { "signature": "", "url": "" }
  }
}
```

Required fields are `version`, `platforms.<target>.url`, `platforms.<target>.signature`. The `signature` field holds **the content of the generated `.sig` file**, not a path. `tauri-action` generates and uploads this file automatically (`uploadUpdaterJson`, default `true`).

There is also a *dynamic* form for a custom server: `{version, pub_date, url, signature, notes}` at the top level, with **HTTP 204 No Content** meaning "no update" (handled explicitly: `if StatusCode::NO_CONTENT == res.status() { return Ok(None) }`).

**Client flow.**

```js
import { check } from '@tauri-apps/plugin-updater';
import { relaunch } from '@tauri-apps/plugin-process';

const update = await check();
if (update?.available) {
  await update.downloadAndInstall();
  await relaunch();
}
```

Capability needed: `"permissions": ["updater:default"]` (which is `allow-check` + `allow-download` + `allow-install` + `allow-download-and-install`). Relaunch additionally needs the `process` plugin.

**Windows install behaviour.** `installMode` options are `passive` (progress bar, no interaction — **default and recommended**), `basicUi`, `quiet`. Critically, from the docs: *"On Windows the application is automatically exited when the install step is executed due to a limitation of Windows installers."* So the sequence is: download → app force-exits → NSIS runs passively → app comes back. Total ~10–20 s for a 5–15 MB installer.

`bundle.windows.nsis.installMode` defaults to **`currentUser`** — "Install the app by default in a directory that doesn't require Administrator access." So **no UAC prompt on update**. Keep it that way; `perMachine` and even `both` require elevation.

**Prefer NSIS over MSI for the updater.** Set `updaterJsonPreferNsis: true` on `tauri-action` so `latest.json` points at `setup.exe` rather than the `.msi`; MSI updates are clunkier and more likely to need elevation.

#### Electron — `electron-builder` + `electron-updater`

**Setup.** Windows auto-update requires the **NSIS** target (the default). From the docs: *"Squirrel.Windows is not supported."*

```yaml
# electron-builder.yml
win:
  target: nsis
publish:
  provider: github
  private: true        # only needed because the repo is private
```

`electron-builder` generates `latest.yml` and uploads it alongside the installer:

```yaml
version: 1.1.0
files:
  - url: TestApp Setup 1.1.0.exe
    sha512: Dj51I0q8aPQ3ioaz9LMqGYujAYRbDNblAQbodDRXAMxmY6hsHqEl3F6SvhfJj5oPhcqdX1ldsgEvfMNXGUXBIw==
    size: 62021782
stagingPercentage: 10
```

It also emits a `.blockmap` next to the installer, which is what enables **differential download** — the second and later updates only pull changed blocks, which is how a 200 MB Electron app still gets a fast inner loop.

**Client flow.** Two lines in the simplest case:

```ts
import { autoUpdater } from "electron-updater"
autoUpdater.checkForUpdatesAndNotify()
```

Full surface: `checkForUpdates()`, `downloadUpdate()`, `quitAndInstall()`, `installPendingUpdateIfAvailable()`, properties `autoDownload` and `autoInstallEvent` (`"manual" | "onQuit" | "onNextLaunch"`, default `"onQuit"`), and events `checking-for-update`, `update-available`, `update-not-available`, `download-progress`, `update-downloaded`, `update-cancelled`, `error`.

**Dev-mode caveat.** The updater does not run from an unpackaged app unless you add a `dev-app-update.yml` at the project root and set `autoUpdater.forceDevUpdateConfig = true`, and even then the docs say *"it is not recommended, better to test auto-update for installed application (especially on Windows)."* For this project that is fine — Windows testing is via the installed app by definition.

Staged rollouts exist via `stagingPercentage` in `latest.yml`; irrelevant for one user, but note the associated warning: to pull a bad release you **must** increment the version *higher*, not republish the same one.

#### .NET — Velopack (not Squirrel.Windows, not ClickOnce)

**Current state of the field.** Squirrel.Windows has *"fallen into a state of disrepair"*; Velopack is [its declared successor](https://docs.velopack.io/migrating/squirrel) (`Squirrel.SquirrelAwareApp` → `Velopack.VelopackApp.Build().Run()`), written in Rust for native performance, with delta packages. ClickOnce still exists and is still documented for .NET 5+ (via `dotnet-mage.exe` / the Publish profile), but it is legacy shaped: Code Access Security is unsupported on modern .NET, updates are manifest-and-sibling-folder based, and the flow is designed around Visual Studio's Publish Wizard rather than CI. For a 2026 greenfield .NET desktop app, **Velopack is the answer**; the other two are only relevant as migration sources.

**Setup.** One line at the top of `Main`:

```csharp
static void Main(string[] args) {
    VelopackApp.Build().Run();
    // ... your startup
}
```

**Client flow.**

```csharp
var mgr = new UpdateManager(new GithubSource("https://github.com/Furizaa/poe-graft", accessToken, prerelease: false));

var newVersion = await mgr.CheckForUpdatesAsync();
if (newVersion == null) return;
await mgr.DownloadUpdatesAsync(newVersion);
mgr.ApplyUpdatesAndRestart(newVersion);
```

`vpk pack` emits `Setup.exe`, a full `.nupkg`, a delta `.nupkg` (when a previous release was downloaded first — that is what `vpk download github` is for), `Portable.zip`, and `releases.{channel}.json` as the feed index. `UpdateManager` has special support for GitHub Releases to find deltas across previous releases — which requires **one full package + one delta per release**.

Install location is `%LocalAppData%\{packId}`, per-user, **no admin rights**.

### 4. Signing — two completely different questions

These get conflated constantly. They are unrelated.

#### (a) What the *updater* cryptographically requires to accept an update

| Stack | Requirement | Cost |
|---|---|---|
| **Tauri v2** | **A minisign keypair. Mandatory.** The docs are explicit: *"Tauri's updater needs a signature to verify that the update is from a trusted source. **This cannot be disabled.**"* Public key in `tauri.conf.json`, private key as `TAURI_SIGNING_PRIVATE_KEY` in CI. | **Free.** Self-generated by `tauri signer generate`. No CA, no certificate, no Authenticode. |
| **Electron** | A SHA-512 hash in `latest.yml`, generated automatically by `electron-builder`. Authenticode verification is **conditional**: `NsisUpdater` reads `publisherName` from `app-update.yml`, and `if (publisherName == null) return null` — a `null` return means the check passes without validating, so **an unsigned installer proceeds to installation**. ([NsisUpdater.ts](https://github.com/electron-userland/electron-builder/blob/master/packages/electron-updater/src/NsisUpdater.ts)) | Free. |
| **Velopack** | Package hash verification. No Authenticode requirement — the [signing docs](https://docs.velopack.io/packaging/signing) frame signing purely as a SmartScreen/AV reputation concern, never as a functional gate. | Free. |

Note the asymmetry: **on macOS, electron-updater *does* require signing** (*"macOS application must be signed in order for auto updating to work"*). Irrelevant here — the Mac is a dev machine only, per the map.

#### (b) What Windows SmartScreen / Defender wants

An **Authenticode code-signing certificate**. OV certs run ~$200–400/year and now require HSM/token key storage; [Azure Artifact Signing / Trusted Signing](https://docs.velopack.io/packaging/signing) is ~$10/month and is what Velopack's docs call *"the most effective way to code-sign your product"* because it gives instant SmartScreen reputation without hardware. With an OV cert instead, per the same docs: *"People get smart screen warnings for a while until the reputation on that file increases."*

None of this is required for anything to *work*.

#### The answer to the ticket's question

**Yes — an unsigned app can auto-update itself on Windows for a single private user, on all three stacks.**

The friction is confined to **one moment**: the very first manual install. You download `Setup.exe` from the GitHub releases page in a browser, the browser attaches a Mark-of-the-Web / `Zone.Identifier`, and SmartScreen shows "Windows protected your PC". You click *More info → Run anyway* once, and you are done forever.

Subsequent updater-driven installs are materially different:
- The installer is fetched by the app's own HTTP client (reqwest / Electron net / Velopack's Rust downloader), which does **not** attach Mark-of-the-Web, so the SmartScreen shell prompt generally does not fire on the silent/passive install.
- All three default to **per-user installs** — Tauri NSIS `installMode: "currentUser"`, Velopack `%LocalAppData%\{packId}`, electron-builder NSIS per-user — so there is **no UAC prompt** either.

**Real residual risk, and it applies specifically to this app:** Defender's heuristic/behavioural engine can still quarantine an unknown unsigned executable, and "unsigned binary that installs a global low-level keyboard hook and suppresses mouse clicks" is precisely the silhouette those heuristics are tuned for. This is the one place where being unsigned could cost you a debugging session on a machine with no dev tools.

**Mitigation, in order of cost:** add a Defender folder exclusion for the install directory on the gaming PC (free, 30 seconds, sufficient for one machine); if it recurs, Azure Trusted Signing at ~$10/month wired into the workflow as a signing step. Do the exclusion now, defer the certificate until something actually breaks.

### 5. Private repositories — the single biggest gotcha

I verified this empirically against `Furizaa/poe-graft` rather than trusting folklore: created a temporary prerelease with a `latest.json` asset, probed both URL shapes with a valid token, then deleted the release and tag.

```
# https://github.com/Furizaa/poe-graft/releases/download/<tag>/latest.json
Authorization: Bearer <valid token>   → HTTP 404
Authorization: token  <valid token>   → HTTP 404
no auth                                → HTTP 404

# https://api.github.com/repos/Furizaa/poe-graft/releases/assets/501124346
Authorization: Bearer <valid token>
Accept: application/octet-stream       → HTTP 200, body "hello-asset-auth-test"
```

**So: on a private repo, `browser_download_url` is unreachable programmatically no matter what token you present.** It is reachable only from a browser with a logged-in GitHub *session*. The only programmatic path is the [REST asset endpoint](https://docs.github.com/en/rest/releases/assets) with `Accept: application/octet-stream` — and that URL is keyed by a numeric **asset ID that changes on every release**. There is no stable, name-based private asset URL.

That single fact splits the three stacks apart.

#### Tauri — does not work out of the box

The updater endpoint is a **static URL** baked into `tauri.conf.json`. It *can* send auth: `check({ headers: { Authorization: 'Bearer <token>' } })` in JS, or `.header("Authorization", "Bearer <token>")` on `updater_builder()` in Rust — and I confirmed from [`updater.rs`](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/updater.rs) that those headers are applied to **both** the check request *and* the artifact download (both `check()` and `download()` do `let mut headers = self.headers.clone()` before `.get(url).headers(headers)`). Tauri also defaults `Accept` only *if not already set*, so you could override it to `application/octet-stream`.

But there is nothing stable to point the static endpoint *at*. Options, best first:

1. **Make the repo public.** Endpoint becomes a plain public URL, zero auth, plus faster runners and free minutes. Recommended.
2. **Keep source private, publish to a public `poe-graft-releases` repo.** `tauri-action` has `owner` and `repo` inputs (confirmed in its `action.yml`), so this is a two-line change plus a PAT with `contents: write` on the releases repo. The installer binary becomes public; the source does not.
3. **A tiny auth proxy** (Cloudflare Worker) holding a fine-grained PAT, resolving `/latest.json` and `/download/<name>` to API asset IDs and streaming them. Works, but you have now added infrastructure to your inner loop — a second thing that can break silently, on the far side of a machine you cannot debug.
4. **A dynamic endpoint** you write yourself. Same objection, more work.

#### Electron — works natively

[`providerFactory.ts`](https://github.com/electron-userland/electron-builder/blob/master/packages/electron-updater/src/providerFactory.ts) selects the provider at runtime:

```typescript
case "github": {
  const githubOptions = data as GithubOptions
  const token = (githubOptions.private ? process.env["GH_TOKEN"] || process.env["GITHUB_TOKEN"] : null) || githubOptions.token
  if (token == null) {
    return new GitHubProvider(githubOptions, updater, runtimeOptions)
  } else {
    return new PrivateGitHubProvider(githubOptions, updater, token, runtimeOptions)
  }
}
```

`PrivateGitHubProvider` does exactly the right thing: `authorization: token ${this.token}` against `/repos/{owner}/{repo}/releases[/latest]` with `Accept: application/vnd.github.v3+json` for metadata, finds the channel file by name (`releaseInfo.assets.find(it => it.name === channelFile)`), and downloads assets via `asset.url` (the API URL) with `Accept: application/octet-stream`.

**Does the token leak into the client? Yes, necessarily.** The docs say the token is set *"on user machine"*, and the design is flagged: *"Private GitHub provider only for [very special] cases — not intended and not suitable for all users."* For a single private user on one machine this is an acceptable trade, but mitigate it: use a **fine-grained PAT scoped to `contents: read` on this one repository only**, not a classic `repo`-scope token. Rate limit is 5,000 req/hour and *"an update check uses up to 3 requests per check"* — you could hammer the button ~27 times a minute forever.

#### Velopack — works natively

`new GithubSource(repoUrl, accessToken, prerelease)` — the accessToken is *"required for private repositories"*. Same embedded-token trade-off and same fine-grained-PAT mitigation as Electron. On the CI side, append `--token ${{ secrets.GITHUB_TOKEN }}` to **both** `vpk download github` and `vpk upload github`.

### 6. On-demand update checks (a button, not launch-only)

All three support this cleanly. This matters more than it sounds: a launch-only check means every test cycle is `quit → relaunch → wait → maybe it updated`, which doubles the perceived latency and makes "did the build land?" ambiguous.

- **Tauri.** `check()` is just an async call — wire it to a button. `update.downloadAndInstall(onEvent)` yields `Started` / `Progress` / `Finished` events for a progress bar. Then `relaunch()`. Remember the app force-exits during the Windows install step.
- **Electron.** Set `autoUpdater.autoDownload = false`, expose `checkForUpdates()` / `downloadUpdate()` / `quitAndInstall()` over IPC, drive the UI from the `update-available` / `download-progress` / `update-downloaded` events.
- **Velopack.** `CheckForUpdatesAsync()` → `DownloadUpdatesAsync(newVersion, progress)` → `ApplyUpdatesAndRestart(newVersion)`, straight off a click handler.

**Build the diagnostics alongside the button.** On a machine with no dev environment, the updater is a black box unless you make it talk. Minimum viable on-device diagnostics — this is cheap now and expensive later:

- The running app version, always visible.
- A "Check for updates" button showing: last check time, the version the feed reports, and the raw error string on failure.
- A link to the GitHub Actions run that produced the installed version (inject `github.run_id` at build time).
- Updater logs to a file you can read over the network or paste into a chat.

Without this, a failing updater on a private repo looks identical to a failing build, which looks identical to "I forgot to bump the version" — three different problems with the same symptom, on the one machine you cannot attach a debugger to.

### 7. Version-bump trigger: tag-driven vs manifest-on-merge

The constraint is simple: **the updater compares versions, so every build you want to test must carry a strictly higher version than what is installed.** If it does not, nothing happens and the app tells you "up to date" — the single most confusing failure mode in this loop.

**Option A — tag-driven.** Push `v0.2.0`, workflow builds. Explicit, standard, gives clean release history. Costs one manual step per test cycle (`git tag && git push --tags` from the Mac) and you will forget it. Also, the official `tauri-action` example uses `releaseDraft: true` — **drafts are invisible to `releases/latest/download/`**, so that example as-written silently breaks the updater. Set `releaseDraft: false` and `prerelease: false`.

**Option B — version in a manifest, bumped by hand on each merge.** `tauri.conf.json` / `package.json` / `.csproj`. Same forgetting problem, plus merge conflicts on the version line.

**Option C (recommended) — CI derives the version on every push to `main`.**

Keep a human-meaningful base in the manifest (`0.1.0`) and let the workflow overwrite the patch with `github.run_number`, which is monotonic per workflow and therefore always a valid semver bump. Write it into the manifest **in CI only** — no commit back to `main`, no bot noise, no push loop.

- **Tauri:** patch `src-tauri/tauri.conf.json` with `jq`/node before `tauri-action`, then `tagName: v__VERSION__`. `tauri-action` reads the version from the config and substitutes `__VERSION__`.
- **Electron:** `npm version --no-git-tag-version 0.1.$RUN` then `electron-builder --win --publish always`.
- **Velopack:** `vpk pack -v 0.1.$RUN`.

Result: **merge is the only action you take.** Five minutes later the button in the app offers a new version. That is the loop the ticket is asking for.

Add `workflow_dispatch` for manual reruns, and `paths-ignore: ['docs/**', '**.md']` so documentation commits don't burn Windows minutes.

### 8. Caveats that will bite, in rough order of likelihood

1. **`releaseDraft: true` breaks the updater.** The `latest/download/` URL skips drafts *and* prereleases. Both must be false.
2. **You cannot test the updater with one release.** Version 1 must be installed by hand (browser → releases page → SmartScreen → Run anyway). The updater only becomes testable once release N+1 exists. Plan for two builds before you learn anything.
3. **Getting build 1 onto the box, private repo.** `browser_download_url` 404s for tooling but works fine in a logged-in browser. Simplest path: sign into GitHub in the browser on the gaming PC and download from the releases page. (Or `gh release download` if you install and auth `gh` there — but that is a dev tool on a machine that is meant to have none.)
4. **Cold-cache cliff (Tauri).** Any `Cargo.lock` change → 12–18 min instead of 5. Batch dependency work.
5. **Version didn't increase → silent no-op.** Option C above removes this class of bug entirely.
6. **Defender heuristics vs a global keyboard hook.** Add the folder exclusion pre-emptively.
7. **Actions minutes on a private repo.** Windows bills at $0.010/min vs Linux $0.006/min, against 2,000 (Free) / 3,000 (Pro) included minutes. A 6-minute build ≈ 6 minutes charged; several hundred builds/month is fine, but it is a real meter. Public repo → unlimited.
8. **`createUpdaterArtifacts: true`** — forget it and `latest.json` never appears, with no obvious error.
9. **Electron + Squirrel.Windows is unsupported.** Use the default NSIS target or `latest.yml` is never produced and `autoUpdater` errors.
10. **MSI vs NSIS for Tauri updates.** Set `updaterJsonPreferNsis: true`.

---

## Recommended workflow sketch

Also written standalone as [`docs/research/workflow-sketch.yml`](./workflow-sketch.yml).

This is the Tauri v2 variant, assuming the recommendation above (public repo, or a public releases repo via the commented `owner`/`repo` inputs). Auto-versioned from `github.run_number`, so **merging to `main` is the only manual step**.

```yaml
name: build-windows

on:
  push:
    branches: [main]
    paths-ignore:
      - 'docs/**'
      - '**.md'
  workflow_dispatch:

concurrency:
  # A newer merge supersedes an in-flight build — keeps the loop tight.
  group: build-windows
  cancel-in-progress: true

permissions:
  contents: write

jobs:
  build:
    runs-on: windows-latest
    steps:
      - uses: actions/checkout@v4

      # --- Derive a monotonically increasing version and write it into the
      # --- manifest in CI only. No commit back to main.
      - name: Set version from run number
        id: version
        shell: bash
        run: |
          VERSION="0.1.${{ github.run_number }}"
          echo "version=$VERSION" >> "$GITHUB_OUTPUT"
          node -e "
            const fs = require('fs');
            const p = 'src-tauri/tauri.conf.json';
            const c = JSON.parse(fs.readFileSync(p, 'utf8'));
            c.version = '$VERSION';
            fs.writeFileSync(p, JSON.stringify(c, null, 2));
          "

      - uses: pnpm/action-setup@v4
        with:
          version: 11

      - uses: actions/setup-node@v4
        with:
          node-version: lts/*
          cache: 'pnpm'

      - uses: dtolnay/rust-toolchain@stable

      # The single most important step for round-trip latency.
      # Caches the registry, git db and dependency build artifacts.
      # NOTE: keyed on Cargo.lock — a dependency change forces a cold build.
      - uses: swatinem/rust-cache@v2
        with:
          workspaces: './src-tauri -> target'

      - run: pnpm install --frozen-lockfile

      - uses: tauri-apps/tauri-action@v1
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
          TAURI_SIGNING_PRIVATE_KEY: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY }}
          TAURI_SIGNING_PRIVATE_KEY_PASSWORD: ${{ secrets.TAURI_SIGNING_PRIVATE_KEY_PASSWORD }}
        with:
          tagName: v__VERSION__
          releaseName: 'poe-graft v__VERSION__'
          releaseBody: |
            Automated build of ${{ github.sha }}.
            Run: ${{ github.server_url }}/${{ github.repository }}/actions/runs/${{ github.run_id }}
          # BOTH must be false or `releases/latest/download/latest.json`
          # will not resolve and the updater silently sees nothing.
          releaseDraft: false
          prerelease: false
          # Point latest.json at setup.exe rather than the .msi.
          updaterJsonPreferNsis: true
          args: '--target x86_64-pc-windows-msvc'
          # --- To keep the source private but the releases public, publish
          # --- to a separate public repo and use a PAT with contents:write:
          # owner: Furizaa
          # repo: poe-graft-releases
```

### Electron variant (the private-repo-friendly fallback)

```yaml
      - uses: actions/setup-node@v4
        with:
          node-version: lts/*
          cache: 'npm'

      - name: Cache Electron binaries
        uses: actions/cache@v4
        with:
          path: ~\AppData\Local\electron
          key: electron-${{ runner.os }}-${{ hashFiles('package-lock.json') }}

      # npm ci, not npm install — native modules must rebuild for this ABI.
      - run: npm ci

      - run: npm version --no-git-tag-version 0.1.${{ github.run_number }}

      - run: npx electron-builder --win --publish always
        env:
          GH_TOKEN: ${{ secrets.GITHUB_TOKEN }}
```

with `electron-builder.yml`:

```yaml
win:
  target: nsis          # required — Squirrel.Windows is not supported
publish:
  provider: github
  private: true         # -> PrivateGitHubProvider, API + octet-stream
  releaseType: release  # NOT draft
```

### Velopack variant

```yaml
      - uses: actions/setup-dotnet@v4
        with:
          dotnet-version: 9.0.x

      - run: dotnet publish src/PoeGraft/PoeGraft.csproj -r win-x64 --self-contained -o publish

      - name: Pack and publish
        shell: bash
        run: |
          dotnet tool install -g vpk
          V=0.1.${{ github.run_number }}
          # download the previous release so vpk can build a delta package
          vpk download github --repoUrl https://github.com/${{ github.repository }} --token ${{ secrets.GITHUB_TOKEN }}
          vpk pack -u PoeGraft -v "$V" -p publish
          vpk upload github --repoUrl https://github.com/${{ github.repository }} --token ${{ secrets.GITHUB_TOKEN }} \
            --publish --releaseName "poe-graft $V" --tag "v$V"
```

---

## Required secrets and setup

### Tauri v2 (recommended path)

**One-time, on the Mac:**

```bash
pnpm tauri signer generate -w ~/.tauri/poe-graft.key
```

**GitHub repository secrets:**

| Secret | Value | Notes |
|---|---|---|
| `TAURI_SIGNING_PRIVATE_KEY` | contents of `~/.tauri/poe-graft.key` | Never commit. Losing it means you can never ship another update to an installed app. Back it up outside the repo. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | the passphrase you chose | Required if the key has one. |
| `GITHUB_TOKEN` | — | Auto-issued per run. Sufficient for same-repo releases. |
| `RELEASES_REPO_TOKEN` | fine-grained PAT, `contents: write` on `poe-graft-releases` | **Only** for the separate-public-releases-repo variant. |

**Repository / config setup:**

1. Make `Furizaa/poe-graft` **public** (or create a public `poe-graft-releases`).
2. `tauri.conf.json`: `bundle.createUpdaterArtifacts: true`.
3. `tauri.conf.json`: `plugins.updater.pubkey` = contents of `~/.tauri/poe-graft.key.pub`.
4. `tauri.conf.json`: `plugins.updater.endpoints` = `["https://github.com/Furizaa/poe-graft/releases/latest/download/latest.json"]`.
5. `tauri.conf.json`: `plugins.updater.windows.installMode: "passive"`.
6. Leave `bundle.windows.nsis.installMode` at its `currentUser` default → no UAC on update.
7. Capabilities: add `updater:default` and the `process` plugin permission for `relaunch()`.
8. Workflow permissions: `contents: write`.

**On the Windows gaming PC, one time:**

1. Sign into GitHub in the browser, download `poe-graft_0.1.N_x64-setup.exe` from the releases page.
2. Click through SmartScreen (*More info → Run anyway*). This happens **once**, not per update.
3. Add a Defender folder exclusion for the install directory (`%LOCALAPPDATA%\poe-graft` or wherever NSIS `currentUser` lands it) — pre-emptive insurance against the global-keyboard-hook heuristic.

### Electron variant, additionally

| Secret / value | Where | Notes |
|---|---|---|
| `GH_TOKEN` | GitHub Actions | For `electron-builder --publish`; `secrets.GITHUB_TOKEN` works. |
| A fine-grained PAT, `contents: read` on `poe-graft` only | **on the Windows machine** (env var, or `autoUpdater.setFeedURL({ token })`) | This is the embedded-client-token trade-off. Scope it to the single repo, read-only. |

### Velopack variant, additionally

| Secret / value | Where | Notes |
|---|---|---|
| `GITHUB_TOKEN` | Actions, on **both** `vpk download github` and `vpk upload github` | Deltas need the download step. |
| A fine-grained PAT, `contents: read` on `poe-graft` only | passed to `new GithubSource(url, token, false)` | Same embedded-token trade-off. |

### Optional, defer until needed

Authenticode signing. Only buy this if Defender or SmartScreen actually interferes after the folder exclusion. [Azure Trusted Signing](https://docs.velopack.io/packaging/signing) at ~$10/month is the cheapest route to instant reputation; OV certificates require a reputation-building period and HSM key storage.

---

## Sources

**Tauri v2**
- [Updater plugin](https://v2.tauri.app/plugin/updater) — signature is mandatory, `latest.json` format, dynamic endpoint + 204, `installMode`, custom headers, `createUpdaterArtifacts`, capabilities
- [GitHub Actions pipeline](https://v2.tauri.app/distribute/pipelines/github/) — official workflow, `rust-cache`, `contents: write`
- [CLI reference — `tauri signer`](https://v2.tauri.app/reference/cli)
- [Config reference](https://v2.tauri.app/reference/config/) — `NsisConfig.installMode` default `currentUser`, `webviewInstallMode`
- [`tauri-apps/tauri-action`](https://github.com/tauri-apps/tauri-action) and its [`action.yml`](https://github.com/tauri-apps/tauri-action/blob/dev/action.yml) — `uploadUpdaterJson`, `updaterJsonPreferNsis`, `owner`, `repo`, outputs
- [`plugins-workspace` `updater.rs`](https://github.com/tauri-apps/plugins-workspace/blob/v2/plugins/updater/src/updater.rs) — headers applied to both check and download; URL variable substitution; 204 handling

**Electron**
- [Auto Update](https://github.com/electron-userland/electron-builder/blob/master/website/docs/features/auto-update.md) — private repo + `GH_TOKEN`, `latest.yml`, auto-updatable targets, API/events, dev-mode caveat, rate limits
- [Publish](https://github.com/electron-userland/electron-builder/blob/master/website/docs/publish.md) — `--publish` values, auto rules, `GITHUB_RELEASE_TOKEN`
- [GitHub Actions](https://github.com/electron-userland/electron-builder/blob/master/website/docs/features/github-actions.md) — workflow, Electron binary cache paths, `npm ci`
- [`providerFactory.ts`](https://github.com/electron-userland/electron-builder/blob/master/packages/electron-updater/src/providerFactory.ts) — public vs private provider selection
- [`PrivateGitHubProvider.ts`](https://github.com/electron-userland/electron-builder/blob/master/packages/electron-updater/src/providers/PrivateGitHubProvider.ts) — API endpoints, Accept headers, asset resolution
- [`NsisUpdater.ts`](https://github.com/electron-userland/electron-builder/blob/master/packages/electron-updater/src/NsisUpdater.ts) — `verifyUpdateCodeSignature`, `publisherName == null` skips verification
- [v27 breaking changes](https://github.com/electron-userland/electron-builder/blob/master/website/docs/migration/v27-breaking-changes.md) — `vPrefixedTagName` → `tagNamePrefix`

**.NET**
- [Velopack — integrating overview](https://docs.velopack.io/integrating/overview) and [update sources](https://docs.velopack.io/integrating/update-sources) — `GithubSource`, token required for private repos
- [Velopack — GitHub Actions](https://docs.velopack.io/distributing/github-actions) — full workflow, `--token` for private repos
- [Velopack — signing](https://docs.velopack.io/packaging/signing) — signing is a SmartScreen concern, not a functional gate
- [Velopack — packaging overview](https://docs.velopack.io/packaging/overview) and [installer](https://docs.velopack.io/packaging/installer) — artifacts, `%LocalAppData%\{packId}`, no admin
- [Velopack — migrating from Squirrel](https://docs.velopack.io/migrating/squirrel) — Squirrel.Windows "in a state of disrepair"
- [ClickOnce Deployment and Security](https://learn.microsoft.com/en-us/visualstudio/deployment/clickonce-security-and-deployment) — CAS unsupported on modern .NET, `dotnet-mage.exe`, manifest-based updates

**GitHub Actions / REST**
- [GitHub-hosted runners](https://docs.github.com/en/actions/reference/runners/github-hosted-runners) — 4 vCPU/16 GB public vs 2 vCPU/8 GB private; free on public repos
- [Actions minute multipliers](https://docs.github.com/en/billing/reference/actions-minute-multipliers) — Windows $0.010/min vs Linux $0.006/min
- [REST: release assets](https://docs.github.com/en/rest/releases/assets) — `Accept: application/octet-stream` is the only programmatic download path
- Community discussions confirming `browser_download_url` is browser-session-only: [#47453](https://github.com/orgs/community/discussions/47453), [#110870](https://github.com/orgs/community/discussions/110870)
- Empirical probe against `Furizaa/poe-graft` (temporary release, since deleted) — results in [§5](#5-private-repositories--the-single-biggest-gotcha)
