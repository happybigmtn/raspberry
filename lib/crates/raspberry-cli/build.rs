fn main() {
    println!("cargo:rerun-if-changed=../../../.git/HEAD");

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
