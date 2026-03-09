use tracing::{debug, info};

use arc_github::{self as github_app, ssh_url_to_https, GitHubAppCredentials};

/// Generate a PR body from the diff and goal using an LLM.
pub async fn generate_pr_body(diff: &str, goal: &str, model: &str) -> Result<String, String> {
    let system = "Write a concise PR description summarizing the changes.".to_string();

    // Truncate diff to fit context windows (~50k chars)
    let max_diff_len = 50_000;
    let truncated_diff = if diff.len() > max_diff_len {
        &diff[..max_diff_len]
    } else {
        diff
    };

    let prompt = format!("Goal: {goal}\n\nDiff:\n```\n{truncated_diff}\n```");

    let params = arc_llm::generate::GenerateParams::new(model)
        .system(system)
        .prompt(prompt);

    let result = arc_llm::generate::generate(params)
        .await
        .map_err(|e| format!("LLM generation failed: {e}"))?;

    Ok(result.response.text())
}

/// Optionally open a pull request after a successful workflow run.
///
/// Returns `Ok(Some(html_url))` if a PR was created, `Ok(None)` if the diff
/// was empty, or `Err` on failure.
pub async fn maybe_open_pull_request(
    creds: &GitHubAppCredentials,
    origin_url: &str,
    base_branch: &str,
    head_branch: &str,
    goal: &str,
    diff: &str,
    model: &str,
) -> Result<Option<String>, String> {
    if diff.is_empty() {
        debug!("Empty diff, skipping pull request creation");
        return Ok(None);
    }

    let https_url = ssh_url_to_https(origin_url);
    let (owner, repo) = github_app::parse_github_owner_repo(&https_url)?;

    let body = generate_pr_body(diff, goal, model).await?;

    let (url, pr_number) = github_app::create_pull_request(
        creds,
        &owner,
        &repo,
        base_branch,
        head_branch,
        goal,
        &body,
    )
    .await?;

    info!(pr_url = %url, pr_number, "Pull request created");

    Ok(Some(url))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn empty_diff_returns_none() {
        let creds = GitHubAppCredentials {
            app_id: "123".to_string(),
            private_key_pem: "unused".to_string(),
        };
        let result = maybe_open_pull_request(
            &creds,
            "https://github.com/owner/repo.git",
            "main",
            "arc/run/123",
            "Fix bug",
            "",
            "claude-sonnet-4-20250514",
        )
        .await;
        assert!(result.is_ok());
        assert!(result.unwrap().is_none());
    }
}
