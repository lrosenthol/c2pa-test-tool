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

// These three tests reflect the known `validationTime` limitation: the test assets use
// a test PKI with certificates valid in 2001, but c2pa-rs validates against the current
// system clock. We assert that validation runs successfully and the failure is specifically
// `signingCredential.expired` — not some other unexpected error. When c2pa-rs gains
// clock-override support, update these tests to assert `overall_pass` instead.

fn assert_expired_cert_failure(yaml_path: &Path) {
    let result = crtool::validation::run_validation(yaml_path);
    assert!(result.is_ok(), "run_validation error: {:?}", result);
    let report = result.unwrap();
    assert!(
        !report.overall_pass,
        "Expected FAIL (expired certs) but got PASS — validationTime may now be supported; \
         update this test to assert overall_pass"
    );
    for m in &report.manifests {
        assert!(
            m.actual_failures
                .iter()
                .any(|c| c == "signingCredential.expired"),
            "Expected signingCredential.expired in failures (clock mismatch), got: {:?}",
            m.actual_failures
        );
    }
}

#[test]
fn test_run_validation_png_valid() {
    assert_expired_cert_failure(Path::new("tests/validation/png_valid.yaml"));
}

#[test]
fn test_run_validation_mp3_valid() {
    assert_expired_cert_failure(Path::new("tests/validation/mp3_valid.yaml"));
}

#[test]
fn test_run_validation_mp4_valid() {
    assert_expired_cert_failure(Path::new("tests/validation/mp4_valid.yaml"));
}
