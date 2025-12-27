use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn validate_yaml_fixture_passes_schema() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = manifest_dir.join("tests/fixtures/manual.yml");
    let yaml = fs::read_to_string(fixture_path).expect("fixture should load");

    mandate::validate_yaml_with_schema_str(&yaml, mandate::BUILTIN_SCHEMA)
        .expect("fixture should validate against schema");
}

#[test]
fn validate_yaml_valid_fixture_file() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = manifest_dir.join("tests/fixtures/valid_manual.yaml");
    let yaml = fs::read_to_string(fixture_path).expect("valid fixture should load");

    mandate::validate_yaml_with_schema_str(&yaml, mandate::BUILTIN_SCHEMA)
        .expect("valid fixture should validate against schema");
}

#[test]
fn validate_yaml_invalid_fixture_file() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = manifest_dir.join("tests/fixtures/invalid_manual.yaml");
    let yaml = fs::read_to_string(fixture_path).expect("invalid fixture should load");

    let err = mandate::validate_yaml_with_schema_str(&yaml, mandate::BUILTIN_SCHEMA)
        .expect_err("invalid fixture should fail schema validation");
    assert!(matches!(err, mandate::MandateError::Schema(_)));
}

#[test]
fn validate_yaml_rejects_invalid_shape() {
    let err = mandate::validate_yaml_with_schema_str("[]", mandate::BUILTIN_SCHEMA)
        .expect_err("array should not match schema");
    assert!(matches!(err, mandate::MandateError::Schema(_)));
}

#[test]
fn validate_yaml_allows_external_schema() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let fixture_path = manifest_dir.join("tests/fixtures/manual.yml");
    let yaml = fs::read_to_string(fixture_path).expect("fixture should load");
    let mut schema_path = std::env::temp_dir();
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time should be monotonic")
        .as_nanos();
    schema_path.push(format!("mandate-schema-{nanos}.yml"));
    fs::write(&schema_path, mandate::BUILTIN_SCHEMA).expect("schema write");

    let result = mandate::validate_yaml_with_schema(&yaml, &schema_path);
    let _ = fs::remove_file(&schema_path);
    result.expect("schema path should validate");
}
