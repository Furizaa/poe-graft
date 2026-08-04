//! What built this binary, baked in at compile time.
//!
//! On a machine with no dev environment, "is this actually the build I just merged?" is the
//! question you cannot otherwise answer. A version number alone does not answer it — a failed
//! build, a superseded build and a forgotten version bump all look like "still the old
//! version". The Actions run number does answer it.

use serde::Serialize;

/// Provenance for the running binary.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BuildInfo {
    /// App version. This is the `tauri.conf.json` value — the one the updater compares — not
    /// `CARGO_PKG_VERSION`, which CI deliberately leaves alone so that bumping the version does
    /// not rewrite `Cargo.lock` and cold-build the whole dependency tree.
    pub version: String,
    /// Short commit sha, or `"local"` for a build made on the development machine.
    pub commit: String,
    /// Actions run number, i.e. the patch component CI derived the version from.
    pub run_number: Option<&'static str>,
    /// Link to the Actions run that produced this binary, when there was one.
    pub run_url: Option<String>,
    /// Which `Platform` implementation is wired in: `"windows"` or `"stub"`.
    pub platform: &'static str,
}

impl BuildInfo {
    /// Assemble the provenance. `version` comes from Tauri's package info so it tracks
    /// `tauri.conf.json`; everything else is compiled in from the CI environment.
    pub fn new(version: String, platform: &'static str) -> Self {
        let repository = option_env!("GITHUB_REPOSITORY");
        let run_id = option_env!("GITHUB_RUN_ID");

        Self {
            version,
            commit: option_env!("GITHUB_SHA")
                .map(|sha| sha.chars().take(7).collect())
                .unwrap_or_else(|| "local".to_string()),
            run_number: option_env!("GITHUB_RUN_NUMBER"),
            run_url: match (repository, run_id) {
                (Some(repo), Some(id)) => {
                    Some(format!("https://github.com/{repo}/actions/runs/{id}"))
                }
                _ => None,
            },
            platform,
        }
    }

    /// One line for the log, so the log itself says which build wrote it.
    pub fn summary(&self) -> String {
        format!(
            "poe-graft {} · commit {} · platform {} · run {}",
            self.version,
            self.commit,
            self.platform,
            self.run_number.unwrap_or("local")
        )
    }
}
