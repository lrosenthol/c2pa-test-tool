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

use crtool::validation::{EmptyObject, StatusCodeSet, StatusCodesExpectations};

fn exp_is_empty() -> StatusCodesExpectations {
    StatusCodesExpectations {
        is_empty: Some(EmptyObject {}),
        ..Default::default()
    }
}

fn exp_contains_all_of(codes: &[&str]) -> StatusCodesExpectations {
    StatusCodesExpectations {
        contains_all_of: Some(StatusCodeSet {
            codes: codes.iter().map(|s| s.to_string()).collect(),
        }),
        ..Default::default()
    }
}

#[test]
fn test_is_empty_passes_on_empty_set() {
    let actual = vec![];
    let (pass, _) = exp_is_empty().check(&actual);
    assert!(pass);
}

#[test]
fn test_is_empty_fails_on_non_empty_set() {
    let actual = vec!["signingCredential.trusted".to_string()];
    let (pass, _) = exp_is_empty().check(&actual);
    assert!(!pass);
}

#[test]
fn test_contains_all_of_passes_when_all_present() {
    let actual = vec![
        "claimSignature.validated".to_string(),
        "signingCredential.trusted".to_string(),
    ];
    let (pass, _) = exp_contains_all_of(&["claimSignature.validated", "signingCredential.trusted"])
        .check(&actual);
    assert!(pass);
}

#[test]
fn test_contains_all_of_fails_when_one_missing() {
    let actual = vec!["claimSignature.validated".to_string()];
    let (pass, _) = exp_contains_all_of(&["claimSignature.validated", "signingCredential.trusted"])
        .check(&actual);
    assert!(!pass);
}

use std::path::Path;

#[test]
#[ignore = "requires validationTime support (not yet available in c2pa-rs); certs expired relative to current system clock"]
fn test_run_validation_png_valid() {
    let yaml_path = Path::new("tests/validation/png_valid.yaml");
    let result = crtool::validation::run_validation(yaml_path);
    assert!(result.is_ok(), "run_validation failed: {:?}", result);
    let report = result.unwrap();
    assert!(report.overall_pass, "Expected PASS but got FAIL:\n{}", report.summary());
}

#[test]
#[ignore = "requires validationTime support (not yet available in c2pa-rs); certs expired relative to current system clock"]
fn test_run_validation_mp3_valid() {
    let yaml_path = Path::new("tests/validation/mp3_valid.yaml");
    let result = crtool::validation::run_validation(yaml_path);
    assert!(result.is_ok(), "run_validation failed: {:?}", result);
    assert!(result.unwrap().overall_pass);
}

#[test]
#[ignore = "requires validationTime support (not yet available in c2pa-rs); certs expired relative to current system clock"]
fn test_run_validation_mp4_valid() {
    let yaml_path = Path::new("tests/validation/mp4_valid.yaml");
    let result = crtool::validation::run_validation(yaml_path);
    assert!(result.is_ok(), "run_validation failed: {:?}", result);
    assert!(result.unwrap().overall_pass);
}
