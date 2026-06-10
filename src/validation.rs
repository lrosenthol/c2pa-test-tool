/*
Copyright 2025 Adobe. All rights reserved.
This file is licensed to you under the Apache License, Version 2.0 (the "License");
you may not use this file except in compliance with the License. You may obtain a copy
of the License at http://www.apache.org/licenses/LICENSE-2.0

Unless required by applicable law or agreed to in writing, software distributed under
the License is distributed on an "AS IS" BASIS, WITHOUT WARRANTIES OR REPRESENTATIONS
OF ANY KIND, either express or implied. See the License for the specific language
governing permissions and limitations under the License.
*/

use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ValidationTestCase {
    pub description: String,
    pub inputs: Inputs,
    pub manifests: Vec<ManifestExpectation>,
    #[serde(rename = "validatorSpecVersions", default)]
    pub validator_spec_versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Inputs {
    #[serde(rename = "assetPath")]
    pub asset_path: String,
    #[serde(rename = "claimSignerTrustListPaths")]
    pub claim_signer_trust_list_paths: Vec<String>,
    #[serde(rename = "tsaTrustListPaths")]
    pub tsa_trust_list_paths: Vec<String>,
    #[serde(rename = "validationTime", default)]
    pub validation_time: Option<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct ManifestExpectation {
    #[serde(default)]
    pub failures: StatusCodesExpectations,
    #[serde(default)]
    pub successes: StatusCodesExpectations,
    #[serde(default)]
    pub informationals: StatusCodesExpectations,
}

/// Unit struct for the schema's empty-object sentinel `{}`.
/// serde_yaml parses `isEmpty: {}` into this cleanly.
#[derive(Debug, Deserialize, Default)]
pub struct EmptyObject {}

#[derive(Debug, Deserialize, Default)]
pub struct StatusCodesExpectations {
    #[serde(rename = "isEmpty")]
    pub is_empty: Option<EmptyObject>,
    #[serde(rename = "isNotEmpty")]
    pub is_not_empty: Option<EmptyObject>,
    #[serde(rename = "containsExactly")]
    pub contains_exactly: Option<StatusCodeSet>,
    #[serde(rename = "containsAllOf")]
    pub contains_all_of: Option<StatusCodeSet>,
    #[serde(rename = "containsNoneOf")]
    pub contains_none_of: Option<StatusCodeSet>,
    #[serde(rename = "containsAnyOf")]
    pub contains_any_of: Option<Vec<StatusCodeSet>>,
}

#[derive(Debug, Deserialize)]
pub struct StatusCodeSet {
    #[serde(default)]
    pub codes: Vec<String>,
}

impl StatusCodesExpectations {
    /// Returns (passed, Vec<failure_reason>).
    pub fn check(&self, actual: &[String]) -> (bool, Vec<String>) {
        let mut failures = Vec::new();
        let actual_set: std::collections::HashSet<&str> =
            actual.iter().map(String::as_str).collect();

        if self.is_empty.is_some() && !actual.is_empty() {
            failures.push(format!("expected empty, got: {:?}", actual));
        }

        if self.is_not_empty.is_some() && actual.is_empty() {
            failures.push("expected non-empty, got empty set".to_string());
        }

        if let Some(set) = &self.contains_exactly {
            let expected_set: std::collections::HashSet<&str> =
                set.codes.iter().map(String::as_str).collect();
            if actual_set != expected_set {
                failures.push(format!(
                    "expected exactly {:?}, got {:?}",
                    set.codes, actual
                ));
            }
        }

        if let Some(set) = &self.contains_all_of {
            let missing: Vec<&str> = set
                .codes
                .iter()
                .map(String::as_str)
                .filter(|c| !actual_set.contains(c))
                .collect();
            if !missing.is_empty() {
                failures.push(format!("missing required codes: {:?}", missing));
            }
        }

        if let Some(set) = &self.contains_none_of {
            let found: Vec<&str> = set
                .codes
                .iter()
                .map(String::as_str)
                .filter(|c| actual_set.contains(c))
                .collect();
            if !found.is_empty() {
                failures.push(format!("unexpected codes present: {:?}", found));
            }
        }

        if let Some(any_of_sets) = &self.contains_any_of {
            for (i, set) in any_of_sets.iter().enumerate() {
                let has_any = set.codes.iter().any(|c| actual_set.contains(c.as_str()));
                if !has_any {
                    failures.push(format!(
                        "containsAnyOf[{}]: none of {:?} found in actual",
                        i, set.codes
                    ));
                }
            }
        }

        (failures.is_empty(), failures)
    }
}
