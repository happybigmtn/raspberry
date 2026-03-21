use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureKind {
    BranchBackedRunRequired,
    SourceBranchMissing,
    IntegrationConflict,
    IntegrationTargetUnavailable,
    DeterministicVerifyCycle,
    TransientLaunchFailure,
    CapabilityContractMismatch,
    StallWatchdog,
    ProviderPolicyMismatch,
    ProofScriptFailure,
    EnvironmentCollision,
    Unknown,
}

impl fmt::Display for FailureKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::BranchBackedRunRequired => "branch_backed_run_required",
            Self::SourceBranchMissing => "source_branch_missing",
            Self::IntegrationConflict => "integration_conflict",
            Self::IntegrationTargetUnavailable => "integration_target_unavailable",
            Self::DeterministicVerifyCycle => "deterministic_verify_cycle",
            Self::TransientLaunchFailure => "transient_launch_failure",
            Self::CapabilityContractMismatch => "capability_contract_mismatch",
            Self::StallWatchdog => "stall_watchdog",
            Self::ProviderPolicyMismatch => "provider_policy_mismatch",
            Self::ProofScriptFailure => "proof_script_failure",
            Self::EnvironmentCollision => "environment_collision",
            Self::Unknown => "unknown",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FailureRecoveryAction {
    ReplayLane,
    ReplaySourceLane,
    RefreshFromTrunk,
    BackoffRetry,
    RegenerateLane,
    SurfaceBlocked,
}

impl fmt::Display for FailureRecoveryAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            Self::ReplayLane => "replay_lane",
            Self::ReplaySourceLane => "replay_source_lane",
            Self::RefreshFromTrunk => "refresh_from_trunk",
            Self::BackoffRetry => "backoff_retry",
            Self::RegenerateLane => "regenerate_lane",
            Self::SurfaceBlocked => "surface_blocked",
        };
        f.write_str(label)
    }
}

pub fn classify_failure(
    last_error: Option<&str>,
    stderr: Option<&str>,
    stdout: Option<&str>,
) -> Option<FailureKind> {
    let mut combined = String::new();
    for text in [last_error, stderr, stdout].into_iter().flatten() {
        if text.trim().is_empty() {
            continue;
        }
        if !combined.is_empty() {
            combined.push('\n');
        }
        combined.push_str(&text.to_lowercase());
    }
    if combined.is_empty() {
        return None;
    }
    if combined.contains("branch-backed run")
        || combined.contains("run is not branch-backed")
        || combined.contains("not branch-backed")
    {
        return Some(FailureKind::BranchBackedRunRequired);
    }
    if combined.contains("source branch `") && combined.contains("missing locally and on") {
        return Some(FailureKind::SourceBranchMissing);
    }
    if combined.contains("merge conflict")
        || combined.contains("git merge --squash failed")
        || combined.contains("automatic merge failed")
        || combined.contains("could not apply")
        || combined.contains("conflict marker")
        || combined.contains("recorded preimage for")
    {
        return Some(FailureKind::IntegrationConflict);
    }
    if combined.contains("could not read from remote repository")
        || combined.contains("does not appear to be a git repository")
        || (combined.contains("cannot force update the branch")
            && combined.contains("used by worktree"))
        || combined.contains("configure origin/head so direct integration can target trunk")
        || combined.contains("set fabro_trunk_branch")
    {
        return Some(FailureKind::IntegrationTargetUnavailable);
    }
    if combined.contains("cycle detection")
        || combined.contains("deterministic failure cycle detected")
        || combined.contains("deterministic cycle")
        || combined.contains("run is stuck in a cycle")
        || combined.contains("visited 3 times")
        || combined.contains("dependency cycle")
    {
        return Some(FailureKind::DeterministicVerifyCycle);
    }
    if combined.contains("sandbox_init_failed")
        || combined.contains("sandbox init failed")
        || combined.contains("worker process disappeared")
        || combined.contains("sandbox_initializing")
        || combined.contains("cannot set terminal process group")
        || combined.contains("no job control in this shell")
        || combined.contains("error finding codex home")
        || combined.contains("codex_home points to")
        || (combined.contains("could not update path") && combined.contains("codex_home"))
    {
        return Some(FailureKind::TransientLaunchFailure);
    }
    if combined.contains("gatewayunauthorized")
        || combined.contains("lacks 'control' capability")
        || combined.contains("lacks 'observe' capability")
        || combined.contains("device lacks 'control' capability")
        || combined.contains("device lacks 'observe' capability")
        || combined.contains("already paired with different capabilities")
    {
        return Some(FailureKind::CapabilityContractMismatch);
    }
    if combined.contains("stall watchdog") || combined.contains("had no activity for 1800s") {
        return Some(FailureKind::StallWatchdog);
    }
    if combined.contains("provider policy")
        || combined.contains("provider rejected")
        || combined.contains("model policy")
        || combined.contains("unsupported provider")
    {
        return Some(FailureKind::ProviderPolicyMismatch);
    }
    if combined.contains("proof script")
        || combined.contains("verify command")
        || combined.contains("verification command")
        || combined.contains("health command")
        || combined.contains("goal gate unsatisfied for node verify")
        || combined.contains("no retry target")
        || combined.contains("script timed out after")
        || combined.contains("script failed with exit code")
    {
        return Some(FailureKind::ProofScriptFailure);
    }
    if combined.contains("errno 98")
        || combined.contains("address already in use")
        || combined.contains("resource busy")
        || combined.contains("text file busy")
        || combined.contains("port is already allocated")
    {
        return Some(FailureKind::EnvironmentCollision);
    }
    Some(FailureKind::Unknown)
}

pub fn default_recovery_action(kind: FailureKind) -> FailureRecoveryAction {
    match kind {
        FailureKind::BranchBackedRunRequired | FailureKind::SourceBranchMissing => {
            FailureRecoveryAction::ReplaySourceLane
        }
        FailureKind::IntegrationTargetUnavailable => FailureRecoveryAction::ReplayLane,
        FailureKind::IntegrationConflict => FailureRecoveryAction::RefreshFromTrunk,
        FailureKind::EnvironmentCollision
        | FailureKind::StallWatchdog
        | FailureKind::TransientLaunchFailure => FailureRecoveryAction::BackoffRetry,
        FailureKind::DeterministicVerifyCycle
        | FailureKind::CapabilityContractMismatch
        | FailureKind::ProofScriptFailure => FailureRecoveryAction::RegenerateLane,
        FailureKind::Unknown | FailureKind::ProviderPolicyMismatch => {
            FailureRecoveryAction::SurfaceBlocked
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_failure_detects_branch_backed_run_requirements() {
        assert_eq!(
            classify_failure(
                Some("run `01KM` is not branch-backed; rerun it in a branch-backed worktree"),
                None,
                None,
            ),
            Some(FailureKind::BranchBackedRunRequired)
        );
    }

    #[test]
    fn classify_failure_detects_environment_collisions() {
        assert_eq!(
            classify_failure(
                Some("bind failed: Errno 98 address already in use"),
                None,
                None
            ),
            Some(FailureKind::EnvironmentCollision)
        );
    }

    #[test]
    fn classify_failure_detects_integration_conflicts() {
        assert_eq!(
            classify_failure(
                Some("git merge --squash failed in /tmp/worktree: Recorded preimage for 'foo'"),
                None,
                None,
            ),
            Some(FailureKind::IntegrationConflict)
        );
    }

    #[test]
    fn classify_failure_detects_deterministic_failure_cycles() {
        assert_eq!(
            classify_failure(
                Some(
                    "Engine error: deterministic failure cycle detected: signature verify repeated 3 times"
                ),
                None,
                None,
            ),
            Some(FailureKind::DeterministicVerifyCycle)
        );
    }

    #[test]
    fn classify_failure_detects_transient_launch_failures() {
        assert_eq!(
            classify_failure(
                Some("tracked run remained active after its worker process disappeared"),
                None,
                None,
            ),
            Some(FailureKind::TransientLaunchFailure)
        );
        assert_eq!(
            classify_failure(Some("sandbox_init_failed"), None, None,),
            Some(FailureKind::TransientLaunchFailure)
        );
        assert_eq!(
            classify_failure(
                Some("bash: cannot set terminal process group (1): Inappropriate ioctl for device\nbash: no job control in this shell"),
                None,
                None,
            ),
            Some(FailureKind::TransientLaunchFailure)
        );
        assert_eq!(
            classify_failure(
                Some("WARNING: proceeding, even though we could not update PATH: CODEX_HOME points to \"/tmp/fabro_cli_demo_codex_home\", but that path does not exist\nError finding codex home: CODEX_HOME points to \"/tmp/fabro_cli_demo_codex_home\", but that path does not exist"),
                None,
                None,
            ),
            Some(FailureKind::TransientLaunchFailure)
        );
    }

    #[test]
    fn classify_failure_detects_node_visit_cycle_limit_failures() {
        assert_eq!(
            classify_failure(
                Some("Engine error: node \"fixup\" visited 3 times (node limit 3); run is stuck in a cycle"),
                None,
                None,
            ),
            Some(FailureKind::DeterministicVerifyCycle)
        );
    }

    #[test]
    fn classify_failure_detects_capability_contract_mismatch() {
        assert_eq!(
            classify_failure(Some("Error: Client lacks 'control' capability"), None, None,),
            Some(FailureKind::CapabilityContractMismatch)
        );
    }

    #[test]
    fn classify_failure_detects_integration_target_unavailable() {
        assert_eq!(
            classify_failure(
                Some("git push failed: fatal: Could not read from remote repository."),
                None,
                None,
            ),
            Some(FailureKind::IntegrationTargetUnavailable)
        );
        assert_eq!(
            classify_failure(
                Some(
                    "git branch -f target branch failed: fatal: cannot force update the branch 'main' used by worktree at '/repo'"
                ),
                None,
                None,
            ),
            Some(FailureKind::IntegrationTargetUnavailable)
        );
    }

    #[test]
    fn deterministic_recovery_actions_regenerate_lane() {
        assert_eq!(
            default_recovery_action(FailureKind::DeterministicVerifyCycle),
            FailureRecoveryAction::RegenerateLane
        );
        assert_eq!(
            default_recovery_action(FailureKind::CapabilityContractMismatch),
            FailureRecoveryAction::RegenerateLane
        );
        assert_eq!(
            default_recovery_action(FailureKind::ProofScriptFailure),
            FailureRecoveryAction::RegenerateLane
        );
        assert_eq!(
            default_recovery_action(FailureKind::TransientLaunchFailure),
            FailureRecoveryAction::BackoffRetry
        );
        assert_eq!(
            default_recovery_action(FailureKind::IntegrationTargetUnavailable),
            FailureRecoveryAction::ReplayLane
        );
    }

    #[test]
    fn classify_failure_treats_generic_script_failures_as_proof_failures() {
        assert_eq!(
            classify_failure(
                Some("Script failed with exit code: 1\n\n## stdout\nbootstrap output"),
                None,
                None,
            ),
            Some(FailureKind::ProofScriptFailure)
        );
        assert_eq!(
            classify_failure(
                Some("Engine error: goal gate unsatisfied for node verify and no retry target"),
                None,
                None,
            ),
            Some(FailureKind::ProofScriptFailure)
        );
        assert_eq!(
            classify_failure(
                Some("Handler error: Script timed out after 600000ms: set -e\ncargo test"),
                None,
                None,
            ),
            Some(FailureKind::ProofScriptFailure)
        );
    }

    #[test]
    fn classify_failure_detects_stall_watchdogs() {
        assert_eq!(
            classify_failure(
                Some("Engine error: stall watchdog: node \"verify\" had no activity for 1800s"),
                None,
                None,
            ),
            Some(FailureKind::StallWatchdog)
        );
    }
}
