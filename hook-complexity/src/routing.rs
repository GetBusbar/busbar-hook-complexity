// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2026 Busbar Inc and contributors

use crate::config::{Config, MissingConfidence};
use busbar_contract::abi::sdk::hook::{Decoded, Verdict};
use serde::Deserialize;
use serde_json::json;
use std::collections::BTreeMap;
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Classification {
    version: u32,
    label: String,
    confidence: Option<f64>,
    probabilities: Option<BTreeMap<String, f64>>,
    source: String,
    schema: String,
}
pub(crate) fn payload(view: &Decoded<'_>, cfg: &Config) -> Result<Vec<u8>, &'static str> {
    if view.has_tools {
        return Err("unsupported_tool_projection");
    }
    if view
        .signals
        .get(busbar_contract::signal::Signal::RequestTextComplete)
        != Some(&busbar_contract::signal::SignalValue::Bool(true))
    {
        return Err("incomplete_text_projection");
    }
    let prompt = view.prompt.as_ref().ok_or("prompt_not_granted")?;
    let last = prompt
        .messages
        .iter()
        .rposition(|(role, _)| role == "user")
        .ok_or("missing_user_request")?;
    let start = last.saturating_sub(cfg.previous_messages);
    let selected = &prompt.messages[start..=last];
    if selected
        .iter()
        .any(|(role, text)| !matches!(role.as_ref(), "user" | "assistant") || text.is_empty())
    {
        return Err("unsupported_message_projection");
    }
    let messages: Vec<_> = selected
        .iter()
        .map(|(role, text)| json!({"role":role,"text":text}))
        .collect();
    let tiers: Vec<_> = cfg
        .tiers
        .iter()
        .map(|t| json!({"label":t.label,"description":t.description,"examples":t.examples}))
        .collect();
    let bytes = serde_json::to_vec(
        &json!({"version":1,"model":cfg.classifier_model,"messages":messages,"tiers":tiers}),
    )
    .map_err(|_| "input_encoding_failed")?;
    if bytes.len() > cfg.max_input_bytes {
        return Err("input_byte_limit");
    }
    Ok(bytes)
}
pub(crate) fn classify(bytes: &[u8], cfg: &Config) -> Result<String, &'static str> {
    let value: Classification = serde_json::from_slice(bytes).map_err(|_| "malformed_result")?;
    if value.version != 1
        || !matches!(
            (value.source.as_str(), value.schema.as_str()),
            ("jev", "jev-choice-v1") | ("nanojev", "nanojev-choice-v1")
        )
    {
        return Err("unsupported_result_schema");
    }
    if !cfg.tiers.iter().any(|t| t.label == value.label) {
        return Err("unknown_tier");
    }
    if let Some(probabilities) = value.probabilities {
        if probabilities.len() != cfg.tiers.len()
            || cfg
                .tiers
                .iter()
                .any(|t| !probabilities.contains_key(&t.label))
            || probabilities
                .values()
                .any(|p| !p.is_finite() || !(0.0..=1.0).contains(p))
            || (probabilities.values().sum::<f64>() - 1.0).abs() > 1e-6
        {
            return Err("invalid_probability_schema");
        }
        if probabilities
            .values()
            .any(|p| *p > probabilities[&value.label])
        {
            return Err("choice_contradicts_probabilities");
        }
    }
    match value.confidence {
        Some(c) if !c.is_finite() || !(0.0..=1.0).contains(&c) => Err("invalid_confidence"),
        Some(c) if c < cfg.min_confidence => Err("low_confidence"),
        None if matches!(cfg.missing_confidence, MissingConfidence::Abstain) => {
            Err("missing_confidence")
        }
        _ => Ok(value.label),
    }
}
pub(crate) fn verdict(label: &str, view: &Decoded<'_>, cfg: &Config) -> Verdict {
    let Some(tier) = cfg.tiers.iter().find(|t| t.label == label) else {
        return Verdict::Abstain;
    };
    let selected: Vec<_> = view
        .candidates
        .iter()
        .filter(|c| tier.candidates.iter().any(|m| m == c.model.as_ref()))
        .map(|c| c.idx)
        .collect();
    if selected.is_empty() {
        tracing::debug!(reason = "no_authorized_candidates", "complexity abstained");
        Verdict::Abstain
    } else {
        Verdict::Prioritize(selected)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn cfg() -> Config {
        Config::parse(r#"{"classifier_model":"classifier","tiers":[{"label":"easy","description":"simple","candidates":["small"]},{"label":"hard","description":"complex","candidates":["large"]}]}"#).unwrap()
    }
    #[test]
    fn classification_confidence_threshold_is_inclusive_and_missing_abstains() {
        let cfg = cfg();
        for (confidence, accepted) in [
            (json!(0.69), false),
            (json!(0.7), true),
            (json!(0.71), true),
            (json!(null), false),
            (json!(1.1), false),
            (json!(-0.1), false),
        ] {
            let bytes=serde_json::to_vec(&json!({"version":1,"label":"easy","confidence":confidence,"source":"jev","schema":"jev-choice-v1"})).unwrap();
            assert_eq!(classify(&bytes, &cfg).is_ok(), accepted);
        }
    }
    #[test]
    fn classification_malformed_probability_schema_never_becomes_confidence() {
        let mut cfg = cfg();
        cfg.missing_confidence = MissingConfidence::Accept;
        for (probabilities, accepted) in [
            (json!({"easy":0.9,"hard":0.1}), true),
            (json!({"easy":0.9}), false),
            (json!({"easy":0.9,"hard":0.9}), false),
        ] {
            let bytes=serde_json::to_vec(&json!({"version":1,"label":"easy","probabilities":probabilities,"source":"jev","schema":"jev-choice-v1"})).unwrap();
            assert_eq!(classify(&bytes, &cfg).is_ok(), accepted);
        }
        let bytes=br#"{"version":1,"label":"../../arbitrary","confidence":1,"source":"jev","schema":"jev-choice-v1"}"#;
        assert!(classify(bytes, &cfg).is_err());
        let bytes=br#"{"version":1,"label":"easy","confidence":NaN,"source":"jev","schema":"jev-choice-v1"}"#;
        assert!(classify(bytes, &cfg).is_err());
    }
}

#[cfg(test)]
mod projection_tests {
    use super::*;
    use busbar_contract::abi::sdk::hook::{DecodedCandidate, Prompt};
    use busbar_contract::signal::{Signal, SignalBag, SignalValue};
    fn config() -> Config {
        Config::parse(r#"{"classifier_model":"classifier","previous_messages":1,"tiers":[{"label":"easy","description":"simple","candidates":["small","not-authorized"]},{"label":"hard","description":"complex","candidates":["large"]}]}"#).unwrap()
    }
    fn view() -> Decoded<'static> {
        let mut signals = SignalBag::new();
        signals.push(Signal::RequestTextComplete, SignalValue::Bool(true));
        Decoded {
            request_id: 1,
            pool: "pool".into(),
            ingress_dialect: "text".into(),
            message_count: 4,
            total_chars: 20,
            max_tokens: None,
            has_tools: false,
            stream: false,
            signals,
            prompt: Some(Prompt {
                system: None,
                messages: vec![
                    ("user".into(), "old".into()),
                    ("assistant".into(), "context".into()),
                    ("user".into(), "latest".into()),
                    ("assistant".into(), "after".into()),
                ],
                body: None,
            }),
            user: None,
            candidates: vec![candidate(9, "small"), candidate(2, "large")],
            budget_remaining: None,
            budget: vec![],
            session: None,
        }
    }
    fn candidate(idx: usize, model: &'static str) -> DecodedCandidate<'static> {
        DecodedCandidate {
            idx,
            model: model.into(),
            provider: "provider".into(),
            weight: 1,
            context_max: None,
            tier: None,
            cost_per_mtok: None,
            tags: vec![],
            latency_ms: None,
            available_concurrency: 1,
            budget_remaining: None,
            rate_headroom: None,
            signals: SignalBag::new(),
        }
    }
    #[test]
    fn classification_window_is_latest_user_and_bounded_history() {
        let cfg = config();
        let view = view();
        let bytes = payload(&view, &cfg).unwrap();
        let value: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            value["messages"],
            json!([{"role":"assistant","text":"context"},{"role":"user","text":"latest"}])
        );
        assert!(value["tiers"][0].get("candidates").is_none());
    }
    #[test]
    fn classification_skips_ungranted_incomplete_tool_and_oversize_projections() {
        let cfg = config();
        let mut v = view();
        v.prompt = None;
        assert_eq!(payload(&v, &cfg), Err("prompt_not_granted"));
        let mut v = view();
        v.signals = SignalBag::new();
        assert_eq!(payload(&v, &cfg), Err("incomplete_text_projection"));
        let mut v = view();
        v.has_tools = true;
        assert_eq!(payload(&v, &cfg), Err("unsupported_tool_projection"));
        let mut v = view();
        v.prompt.as_mut().unwrap().messages[2].1 = "a".repeat(65_536).into();
        assert_eq!(payload(&v, &cfg), Err("input_byte_limit"));
    }
    #[test]
    fn classification_preference_never_invents_a_candidate() {
        let cfg = config();
        let mut v = view();
        assert_eq!(verdict("easy", &v, &cfg), Verdict::Prioritize(vec![9]));
        v.candidates.remove(0);
        assert_eq!(verdict("easy", &v, &cfg), Verdict::Abstain);
        assert_eq!(verdict("malicious-target", &v, &cfg), Verdict::Abstain);
    }
}
