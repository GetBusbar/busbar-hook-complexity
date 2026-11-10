// SPDX-License-Identifier: Apache-2.0
// Copyright (C) 2026 Busbar Inc and contributors

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Tier {
    pub label: String,
    pub description: String,
    #[serde(default)]
    pub examples: Vec<String>,
    pub candidates: Vec<String>,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MissingConfidence {
    #[default]
    Abstain,
    Accept,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Mode {
    #[default]
    Prefer,
}
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Config {
    #[serde(default)]
    pub mode: Mode,
    pub classifier_model: String,
    pub tiers: Vec<Tier>,
    #[serde(default = "threshold")]
    pub min_confidence: f64,
    #[serde(default)]
    pub missing_confidence: MissingConfidence,
    #[serde(default)]
    pub previous_messages: usize,
    #[serde(default = "input_limit")]
    pub max_input_bytes: usize,
}
fn threshold() -> f64 {
    0.7
}
fn input_limit() -> usize {
    16_384
}
impl Config {
    pub fn parse(settings: &str) -> Result<Self, String> {
        let cfg: Self =
            serde_json::from_str(settings).map_err(|e| format!("complexity settings: {e}"))?;
        cfg.validate()?;
        Ok(cfg)
    }
    fn validate(&self) -> Result<(), String> {
        if self.classifier_model.is_empty() || self.classifier_model.len() > 256 {
            return Err("classifier_model must be a configured target binding name".into());
        }
        if !self.min_confidence.is_finite()
            || self.min_confidence < 0.0
            || self.min_confidence > 1.0
        {
            return Err("min_confidence must be finite in [0, 1]".into());
        }
        if self.previous_messages > 32 || !(256..=65_536).contains(&self.max_input_bytes) {
            return Err("history or input byte limit out of bounds".into());
        }
        if !(2..=32).contains(&self.tiers.len()) {
            return Err("configure 2..32 tiers".into());
        }
        let mut labels = BTreeSet::new();
        for t in &self.tiers {
            if t.label.is_empty()
                || t.label.len() > 128
                || !labels.insert(&t.label)
                || t.description.is_empty()
                || t.description.len() > 4096
                || t.examples.len() > 16
                || t.examples.iter().any(|e| e.len() > 4096)
                || t.candidates.is_empty()
                || t.candidates.len() > 128
                || t.candidates.iter().any(|c| c.is_empty() || c.len() > 256)
            {
                return Err("invalid tier definition or candidate selector".into());
            }
        }
        // Tier definitions count towards the same bounded child request, including JSON escaping.
        if serde_json::to_vec(self).map_err(|e| e.to_string())?.len() > self.max_input_bytes {
            return Err("tier configuration exceeds input byte limit".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn classification_settings_reject_invalid_thresholds_tiers_and_bounds() {
        let good = serde_json::json!({"classifier_model":"classifier","tiers":[{"label":"easy","description":"simple","candidates":["small"]},{"label":"hard","description":"complex","candidates":["large"]}]});
        for value in [
            serde_json::json!(-0.1),
            serde_json::json!(1.1),
            serde_json::json!("NaN"),
        ] {
            let mut bad = good.clone();
            bad["min_confidence"] = value;
            assert!(Config::parse(&bad.to_string()).is_err());
        }
        let mut bad = good.clone();
        bad["tiers"][1]["label"] = serde_json::json!("easy");
        assert!(Config::parse(&bad.to_string()).is_err());
        let mut bad = good.clone();
        bad["previous_messages"] = serde_json::json!(33);
        assert!(Config::parse(&bad.to_string()).is_err());
        let mut bad = good.clone();
        bad["max_input_bytes"] = serde_json::json!(65_537);
        assert!(Config::parse(&bad.to_string()).is_err());
        assert!(matches!(
            Config::parse(&good.to_string()).unwrap().missing_confidence,
            MissingConfidence::Abstain
        ));
    }
}
