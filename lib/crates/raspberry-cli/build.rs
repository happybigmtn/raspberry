fn main() {
    println!("cargo:rerun-if-changed=../../../.git/HEAD");
    println!("cargo:rerun-if-changed=../../../.git/packed-refs");

    if let Some(ref_path) = git_path("HEAD") {
        println!("cargo:rerun-if-changed={ref_path}");
    }
    if let Some(symbolic_ref) = git_symbolic_ref() {
        if let Some(ref_path) = git_path(&symbolic_ref) {
            println!("cargo:rerun-if-changed={ref_path}");
        }
    }

    let sha = std::process::Command::new("git")
        .args(["rev-list", "-1", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if !output.status.success() {
                return None;
            }
            String::from_utf8(output.stdout)
                .ok()
                .map(|stdout| stdout.trim().to_string())
        })
        .unwrap_or_default();
    let short_sha = if sha.len() >= 7 { &sha[..7] } else { &sha };
    println!("cargo:rustc-env=RASPBERRY_GIT_SHA={short_sha}");

    let build_date = chrono::Utc::now().format("%Y-%m-%d").to_string();
    println!("cargo:rustc-env=RASPBERRY_BUILD_DATE={build_date}");
}

fn git_path(path: &str) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--git-path", path])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|stdout| stdout.trim().to_string())
            } else {
                None
            }
        })
}

fn git_symbolic_ref() -> Option<String> {
    std::process::Command::new("git")
        .args(["symbolic-ref", "--quiet", "HEAD"])
        .output()
        .ok()
        .and_then(|output| {
            if output.status.success() {
                String::from_utf8(output.stdout)
                    .ok()
                    .map(|stdout| stdout.trim().to_string())
            } else {
                None
            }
        })
}
