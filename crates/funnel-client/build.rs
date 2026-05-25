use std::env;
use std::fs;
use std::process::Command;

fn main() {
    emit_git_hash();
}

fn emit_git_hash() {
    let hash = run("git", &["rev-parse", "--short", "HEAD"])
        .or_else(|| env::var("FUNNEL_GIT_HASH").ok())
        .unwrap_or_else(|| "unknown".to_string());

    println!("cargo:rustc-env=FUNNEL_GIT_HASH={hash}");

    if let Some(git_dir) = run("git", &["rev-parse", "--git-dir"]) {
        let head_path = format!("{git_dir}/HEAD");
        println!("cargo:rerun-if-changed={head_path}");
        if let Ok(head_ref) = fs::read_to_string(&head_path)
            && let Some(ref_path) = head_ref.strip_prefix("ref: ")
        {
            println!("cargo:rerun-if-changed={git_dir}/{}", ref_path.trim());
        }
    }
    println!("cargo:rerun-if-env-changed=FUNNEL_GIT_HASH");
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    Command::new(cmd)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
}
