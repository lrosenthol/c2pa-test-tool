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
