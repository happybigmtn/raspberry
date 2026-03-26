pub mod catalog;
pub mod model_ref;
pub mod policy;
pub mod provider;
pub mod types;

pub use catalog::{
    Catalog, FallbackTarget, build_fallback_chain, closest_model, default_model,
    default_model_for_provider, default_model_from_env, get_model_info, list_models,
    probe_model_for_provider,
};
pub use model_ref::ModelRef;
pub use policy::{
    automation_chain, automation_fallback_map, automation_fallback_targets,
    automation_primary_target, automation_profile_for_target, AutomationProfile, ModelTarget,
};
pub use provider::Provider;
pub use types::{Model, Model as ModelInfo, ModelCosts, ModelFeatures, ModelLimits};
