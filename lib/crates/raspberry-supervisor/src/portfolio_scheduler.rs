use std::collections::{BTreeMap, BTreeSet};

use crate::plan_registry::{PlanChildRecord, PlanRecord, PlanRegistry, WorkflowArchetype};

/// A candidate child work item eligible for dispatch.
#[derive(Debug, Clone)]
pub struct SchedulableChild {
    pub plan_id: String,
    pub child_id: String,
    pub unit_id: String,
    pub archetype: Option<WorkflowArchetype>,
    pub owned_surfaces: Vec<String>,
    pub dependency_plan_ids: Vec<String>,
    pub priority_score: i64,
}

/// Surface lock claim for a running child.
#[derive(Debug, Clone)]
pub struct SurfaceLock {
    pub unit_id: String,
    pub surfaces: Vec<String>,
}

/// Result of scheduling: which children can run and which are blocked.
#[derive(Debug, Clone)]
pub struct ScheduleResult {
    pub dispatchable: Vec<SchedulableChild>,
    pub blocked_by_dependency: Vec<SchedulableChild>,
    pub blocked_by_surface_conflict: Vec<SchedulableChild>,
    pub blocked_by_budget: Vec<SchedulableChild>,
}

/// Compute which plan children can be dispatched given the current state.
///
/// Filters by: dependency satisfaction, surface lock conflicts, parallelism budget.
/// Prefers: children that unblock the most downstream plans (shared foundations first).
pub fn schedule_portfolio(
    registry: &PlanRegistry,
    running_locks: &[SurfaceLock],
    completed_unit_ids: &BTreeSet<String>,
    max_parallel: usize,
    currently_running: usize,
) -> ScheduleResult {
    let budget = max_parallel.saturating_sub(currently_running);
    let locked_surfaces = running_locks
        .iter()
        .flat_map(|lock| lock.surfaces.iter().cloned())
        .collect::<BTreeSet<_>>();

    // Build a reverse dependency map: plan_id → list of plans that depend on it
    let dependents = build_dependents_map(registry);

    let mut ready = Vec::new();
    let mut blocked_dep = Vec::new();
    let mut blocked_surface = Vec::new();

    for plan in &registry.plans {
        for child in &plan.children {
            let unit_id = child_unit_id(&plan.plan_id, &child.child_id);

            if completed_unit_ids.contains(&unit_id) {
                continue;
            }
            // Parent bootstrap must be complete
            if !completed_unit_ids.contains(&plan.plan_id) {
                blocked_dep.push(make_schedulable(plan, child, &dependents));
                continue;
            }
            // Plan-level dependencies must be complete
            let deps_satisfied = plan
                .dependency_plan_ids
                .iter()
                .all(|dep| completed_unit_ids.contains(dep));
            if !deps_satisfied {
                blocked_dep.push(make_schedulable(plan, child, &dependents));
                continue;
            }
            // Surface lock check
            let conflicts = child
                .owned_surfaces
                .iter()
                .any(|surface| locked_surfaces.contains(surface));
            if conflicts {
                blocked_surface.push(make_schedulable(plan, child, &dependents));
                continue;
            }

            ready.push(make_schedulable(plan, child, &dependents));
        }
    }

    // Sort ready children: highest priority first (unblocks more downstream)
    ready.sort_by(|a, b| b.priority_score.cmp(&a.priority_score));

    let mut dispatchable = Vec::new();
    let mut blocked_budget = Vec::new();
    let mut claimed_surfaces = locked_surfaces;

    for child in ready {
        if dispatchable.len() >= budget {
            blocked_budget.push(child);
            continue;
        }
        // Double-check surface conflict with newly claimed surfaces
        let new_conflict = child
            .owned_surfaces
            .iter()
            .any(|surface| claimed_surfaces.contains(surface));
        if new_conflict {
            blocked_budget.push(child);
            continue;
        }
        for surface in &child.owned_surfaces {
            claimed_surfaces.insert(surface.clone());
        }
        dispatchable.push(child);
    }

    ScheduleResult {
        dispatchable,
        blocked_by_dependency: blocked_dep,
        blocked_by_surface_conflict: blocked_surface,
        blocked_by_budget: blocked_budget,
    }
}

fn child_unit_id(plan_id: &str, child_id: &str) -> String {
    if child_id.starts_with(plan_id) {
        child_id.to_string()
    } else {
        format!("{plan_id}-{child_id}")
    }
}

fn make_schedulable(
    plan: &PlanRecord,
    child: &PlanChildRecord,
    dependents: &BTreeMap<String, Vec<String>>,
) -> SchedulableChild {
    let unit_id = child_unit_id(&plan.plan_id, &child.child_id);
    // Priority: how many downstream plans does this plan's completion unblock?
    let downstream_count = dependents
        .get(&plan.plan_id)
        .map(|deps| deps.len())
        .unwrap_or(0) as i64;
    // Shared foundation gets a boost
    let foundation_boost = if matches!(child.archetype, Some(WorkflowArchetype::Implement))
        && is_shared_surface(&child.owned_surfaces)
    {
        50
    } else {
        0
    };

    SchedulableChild {
        plan_id: plan.plan_id.clone(),
        child_id: child.child_id.clone(),
        unit_id,
        archetype: child.archetype,
        owned_surfaces: child.owned_surfaces.clone(),
        dependency_plan_ids: plan.dependency_plan_ids.clone(),
        priority_score: downstream_count * 10 + foundation_boost,
    }
}

fn is_shared_surface(surfaces: &[String]) -> bool {
    surfaces.iter().any(|s| {
        let lower = s.to_ascii_lowercase();
        lower.contains("core")
            || lower.contains("shared")
            || lower.contains("common")
            || lower.contains("foundation")
            || lower.contains("sdk")
            || lower.contains("lib.rs")
    })
}

fn build_dependents_map(registry: &PlanRegistry) -> BTreeMap<String, Vec<String>> {
    let mut dependents: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for plan in &registry.plans {
        for dep_id in &plan.dependency_plan_ids {
            dependents
                .entry(dep_id.clone())
                .or_default()
                .push(plan.plan_id.clone());
        }
    }
    dependents
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plan_registry::*;
    use std::path::PathBuf;

    fn test_plan(id: &str, deps: &[&str], children: Vec<PlanChildRecord>) -> PlanRecord {
        PlanRecord {
            plan_id: id.to_string(),
            path: PathBuf::from(format!("plans/{id}.md")),
            title: id.to_string(),
            category: PlanCategory::Game,
            composite: !children.is_empty(),
            dependency_plan_ids: deps.iter().map(|d| d.to_string()).collect(),
            mapping_contract_path: None,
            mapping_source: PlanMappingSource::Inferred,
            bootstrap_required: true,
            implementation_required: true,
            declared_child_ids: children.iter().map(|c| c.child_id.clone()).collect(),
            children,
        }
    }

    fn test_child(id: &str, surfaces: &[&str]) -> PlanChildRecord {
        PlanChildRecord {
            child_id: id.to_string(),
            title: None,
            archetype: Some(WorkflowArchetype::Implement),
            lane_kind: None,
            review_profile: None,
            proof_commands: Vec::new(),
            owned_surfaces: surfaces.iter().map(|s| s.to_string()).collect(),
            where_surfaces: None,
            how_description: None,
            state_artifacts: None,
            required_tests: None,
            verification_plan: None,
            rollback_condition: None,
        }
    }

    #[test]
    fn schedule_refuses_conflicting_surface_locks() {
        // #given two children claiming the same surface
        let registry = PlanRegistry {
            plans: vec![test_plan(
                "craps",
                &[],
                vec![
                    test_child("casino-core", &["crates/casino-core/"]),
                    test_child("provably-fair", &["crates/provably-fair/"]),
                ],
            )],
        };
        let running = vec![SurfaceLock {
            unit_id: "craps-casino-core".to_string(),
            surfaces: vec!["crates/casino-core/".to_string()],
        }];
        let completed = BTreeSet::from(["craps".to_string()]);

        // #when scheduling
        let result = schedule_portfolio(&registry, &running, &completed, 5, 1);

        // #then provably-fair is dispatchable, casino-core blocked by its own surface lock
        assert_eq!(result.dispatchable.len(), 1);
        assert_eq!(result.dispatchable[0].child_id, "provably-fair");
        assert_eq!(result.blocked_by_surface_conflict.len(), 1);
        assert_eq!(
            result.blocked_by_surface_conflict[0].child_id,
            "casino-core"
        );
    }

    #[test]
    fn schedule_prefers_shared_foundations() {
        // #given a foundation plan that unblocks two game plans
        let registry = PlanRegistry {
            plans: vec![
                test_plan(
                    "casino-core",
                    &[],
                    vec![test_child("trait-def", &["crates/casino-core/src/lib.rs"])],
                ),
                test_plan(
                    "craps",
                    &["casino-core"],
                    vec![test_child("engine", &["crates/casino-core/src/craps/"])],
                ),
                test_plan(
                    "poker",
                    &["casino-core"],
                    vec![test_child("variant", &["crates/casino-core/src/poker.rs"])],
                ),
            ],
        };
        let completed = BTreeSet::from(["casino-core".to_string()]);

        // #when scheduling with budget=1
        let result = schedule_portfolio(&registry, &[], &completed, 1, 0);

        // #then foundation child (trait-def) is preferred over game children
        assert_eq!(result.dispatchable.len(), 1);
        assert_eq!(result.dispatchable[0].child_id, "trait-def");
    }

    #[test]
    fn schedule_blocks_children_whose_parent_bootstrap_incomplete() {
        // #given parent bootstrap not complete
        let registry = PlanRegistry {
            plans: vec![test_plan(
                "craps",
                &[],
                vec![test_child("casino-core", &["crates/casino-core/"])],
            )],
        };
        let completed = BTreeSet::new();

        // #when scheduling
        let result = schedule_portfolio(&registry, &[], &completed, 5, 0);

        // #then child is blocked by dependency
        assert_eq!(result.dispatchable.len(), 0);
        assert_eq!(result.blocked_by_dependency.len(), 1);
    }
}
