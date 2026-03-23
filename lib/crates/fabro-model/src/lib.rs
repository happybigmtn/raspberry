pub mod catalog;
pub mod policy;
pub mod provider;
pub mod types;

pub use catalog::{
    build_fallback_chain, closest_model, default_model, default_model_for_provider,
    default_model_from_env, get_model_info, list_models, probe_model_for_provider, FallbackTarget,
};
pub use policy::{
    automation_chain, automation_fallback_map, automation_fallback_targets,
    automation_primary_target, automation_profile_for_target, AutomationProfile, ModelTarget,
};
pub use provider::{ModelId, Provider};
pub use types::{ModelCosts, ModelFeatures, ModelInfo, ModelLimits};
