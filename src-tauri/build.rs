fn main() {
    // Build provenance is baked in with `option_env!`, which Cargo does not otherwise know to
    // watch. Without these, a rebuild that differs only in CI metadata would keep the stale
    // values — and the whole point of showing the run number on the gaming PC is that it can be
    // trusted.
    println!("cargo:rerun-if-env-changed=GITHUB_RUN_ID");
    println!("cargo:rerun-if-env-changed=GITHUB_RUN_NUMBER");
    println!("cargo:rerun-if-env-changed=GITHUB_SHA");
    println!("cargo:rerun-if-env-changed=GITHUB_REPOSITORY");

    tauri_build::build()
}
