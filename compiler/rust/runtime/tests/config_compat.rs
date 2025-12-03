use insta::assert_yaml_snapshot;

use reml_runtime::config::{
    compatibility_profile, ConfigCompatibility, KeyPolicy, TrailingCommaMode,
};

#[test]
fn config_compat_strict_json_snapshot() {
    let compat = ConfigCompatibility::strict_json();
    assert_yaml_snapshot!("config_compat_strict_json", compat);
}

#[test]
fn config_compat_toml_relaxed_snapshot() {
    let compat = ConfigCompatibility::relaxed_toml();
    assert_yaml_snapshot!("config_compat_toml_relaxed", compat);
}

#[test]
fn compatibility_profile_helper_handles_aliases() {
    let compat = compatibility_profile("toml-relaxed").expect("profile");
    assert_eq!(compat.unquoted_key, KeyPolicy::AllowAlphaNumeric);
    assert_eq!(
        compat.trailing_comma,
        TrailingCommaMode::ArraysAndObjects
    );
    let err = compatibility_profile("unknown-profile")
        .expect_err("should reject unknown profile");
    assert_eq!(err.requested(), "unknown-profile");
}
