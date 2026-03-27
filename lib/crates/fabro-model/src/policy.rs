use std::collections::HashMap;

use crate::provider::Provider;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AutomationProfile {
    Write,
    Review,
    Synth,
    Unblock,
    DeepReview,
    Adjudication,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModelTarget {
    pub provider: Provider,
    pub model: &'static str,
}

const WRITE_CHAIN: &[ModelTarget] = &[
    ModelTarget {
        provider: Provider::Minimax,
        model: "MiniMax-M2.7-highspeed",
    },
    ModelTarget {
        provider: Provider::Kimi,
        model: "kimi-k2.5",
    },
    ModelTarget {
        provider: Provider::Anthropic,
        model: "claude-opus-4-6",
    },
];

const REVIEW_CHAIN: &[ModelTarget] = &[
    ModelTarget {
        provider: Provider::Kimi,
        model: "kimi-k2.5",
    },
    ModelTarget {
        provider: Provider::Minimax,
        model: "MiniMax-M2.7-highspeed",
    },
    ModelTarget {
        provider: Provider::Anthropic,
        model: "claude-opus-4-6",
    },
];

const SYNTH_CHAIN: &[ModelTarget] = &[
    ModelTarget {
        provider: Provider::Anthropic,
        model: "claude-opus-4-6",
    },
    ModelTarget {
        provider: Provider::OpenAi,
        model: "gpt-5.3-codex",
    },
    ModelTarget {
        provider: Provider::Minimax,
        model: "MiniMax-M2.7-highspeed",
    },
];

const UNBLOCK_CHAIN: &[ModelTarget] = &[ModelTarget {
    provider: Provider::OpenAi,
    model: "gpt-5.4",
}];

const DEEP_REVIEW_CHAIN: &[ModelTarget] = &[
    ModelTarget {
        provider: Provider::Anthropic,
        model: "claude-opus-4-6",
    },
    ModelTarget {
        provider: Provider::OpenAi,
        model: "gpt-5.4",
    },
];

const ADJUDICATION_CHAIN: &[ModelTarget] = &[
    ModelTarget {
        provider: Provider::OpenAi,
        model: "gpt-5.4",
    },
    ModelTarget {
        provider: Provider::Anthropic,
        model: "claude-opus-4-6",
    },
];

pub fn automation_chain(profile: AutomationProfile) -> &'static [ModelTarget] {
    match profile {
        AutomationProfile::Write => WRITE_CHAIN,
        AutomationProfile::Review => REVIEW_CHAIN,
        AutomationProfile::Synth => SYNTH_CHAIN,
        AutomationProfile::Unblock => UNBLOCK_CHAIN,
        AutomationProfile::DeepReview => DEEP_REVIEW_CHAIN,
        AutomationProfile::Adjudication => ADJUDICATION_CHAIN,
    }
}

pub fn automation_primary_target(profile: AutomationProfile) -> ModelTarget {
    automation_chain(profile)[0]
}

pub fn automation_profile_for_target(provider: Provider, model: &str) -> Option<AutomationProfile> {
    for profile in [
        AutomationProfile::Write,
        AutomationProfile::Review,
        AutomationProfile::Synth,
        AutomationProfile::Unblock,
        AutomationProfile::DeepReview,
        AutomationProfile::Adjudication,
    ] {
        let target = automation_primary_target(profile);
        if target.provider == provider && target.model == model {
            return Some(profile);
        }
    }
    None
}

pub fn automation_fallback_targets(profile: AutomationProfile) -> &'static [ModelTarget] {
    let chain = automation_chain(profile);
    if chain.len() <= 1 {
        &[]
    } else {
        &chain[1..]
    }
}

pub fn automation_fallback_map(profile: AutomationProfile) -> HashMap<String, Vec<String>> {
    let primary = automation_primary_target(profile);
    let fallbacks = automation_fallback_targets(profile)
        .iter()
        .map(|target| target.model.to_string())
        .collect::<Vec<_>>();
    if fallbacks.is_empty() {
        return HashMap::new();
    }
    HashMap::from([(primary.provider.as_str().to_string(), fallbacks)])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_profile_orders_minimax_kimi_opus() {
        let chain = automation_chain(AutomationProfile::Write);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].provider, Provider::Minimax);
        assert_eq!(chain[0].model, "MiniMax-M2.7-highspeed");
        assert_eq!(chain[1].provider, Provider::Kimi);
        assert_eq!(chain[1].model, "kimi-k2.5");
        assert_eq!(chain[2].provider, Provider::Anthropic);
        assert_eq!(chain[2].model, "claude-opus-4-6");
    }

    #[test]
    fn review_profile_orders_kimi_minimax_opus() {
        let chain = automation_chain(AutomationProfile::Review);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].provider, Provider::Kimi);
        assert_eq!(chain[0].model, "kimi-k2.5");
        assert_eq!(chain[1].provider, Provider::Minimax);
        assert_eq!(chain[1].model, "MiniMax-M2.7-highspeed");
        assert_eq!(chain[2].provider, Provider::Anthropic);
        assert_eq!(chain[2].model, "claude-opus-4-6");
    }

    #[test]
    fn synth_profile_orders_opus_codex_minimax() {
        let chain = automation_chain(AutomationProfile::Synth);
        assert_eq!(chain.len(), 3);
        assert_eq!(chain[0].provider, Provider::Anthropic);
        assert_eq!(chain[0].model, "claude-opus-4-6");
        assert_eq!(chain[1].provider, Provider::OpenAi);
        assert_eq!(chain[1].model, "gpt-5.3-codex");
        assert_eq!(chain[2].provider, Provider::Minimax);
        assert_eq!(chain[2].model, "MiniMax-M2.7-highspeed");
    }

    #[test]
    fn unblock_profile_orders_gpt_54_only() {
        let chain = automation_chain(AutomationProfile::Unblock);
        assert_eq!(chain.len(), 1);
        assert_eq!(chain[0].provider, Provider::OpenAi);
        assert_eq!(chain[0].model, "gpt-5.4");
    }

    #[test]
    fn deep_review_profile_orders_opus_then_gpt_54() {
        let chain = automation_chain(AutomationProfile::DeepReview);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].provider, Provider::Anthropic);
        assert_eq!(chain[0].model, "claude-opus-4-6");
        assert_eq!(chain[1].provider, Provider::OpenAi);
        assert_eq!(chain[1].model, "gpt-5.4");
    }

    #[test]
    fn adjudication_profile_orders_gpt_54_then_opus() {
        let chain = automation_chain(AutomationProfile::Adjudication);
        assert_eq!(chain.len(), 2);
        assert_eq!(chain[0].provider, Provider::OpenAi);
        assert_eq!(chain[0].model, "gpt-5.4");
        assert_eq!(chain[1].provider, Provider::Anthropic);
        assert_eq!(chain[1].model, "claude-opus-4-6");
    }
}
