fn main() {
    let hash = run("git", &["rev-parse", "--short", "HEAD"])
        .or_else(|| std::env::var("FUNNEL_GIT_HASH").ok())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=FUNNEL_GIT_HASH={hash}");

    // only rerun when git ref changes (new commit, branch switch) or
    // when FUNNEL_GIT_HASH is set externally (nix, ci)
    if let Some(git_dir) = run("git", &["rev-parse", "--git-dir"]) {
        let head_path = format!("{git_dir}/HEAD");
        println!("cargo:rerun-if-changed={head_path}");
        if let Ok(head_ref) = std::fs::read_to_string(&head_path) {
            if let Some(ref_path) = head_ref.strip_prefix("ref: ") {
                println!("cargo:rerun-if-changed={git_dir}/{}", ref_path.trim());
            }
        }
    }
    println!("cargo:rerun-if-env-changed=FUNNEL_GIT_HASH");
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    std::process::Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}
