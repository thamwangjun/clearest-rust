//! Phase 3: ANTHROPIC_AUTH_TOKEN bearer auth resolution + conflict detection.
//!
//! These tests exercise `Config::resolve_anthropic_auth_async()` after the
//! Phase 3 resolver upgrade (D-01..D-09). All env-mutating tests are
//! serialised with `#[serial]` and reset both credential env vars at the
//! top and bottom of each test body to prevent cross-test pollution (D-08).

use claurst_core::{Config, ProviderConfig};
use serial_test::serial;
use std::collections::HashMap;

/// Clear both Anthropic credential env vars. Call at top and bottom of every test.
fn reset_anthropic_env() {
    std::env::remove_var("ANTHROPIC_API_KEY");
    std::env::remove_var("ANTHROPIC_AUTH_TOKEN");
}

/// Build a Config with a custom ProviderConfig for the "anthropic" provider.
fn anthropic_config_with(provider: ProviderConfig) -> Config {
    let mut cfg = Config::default();
    cfg.provider = Some("anthropic".into());
    cfg.provider_configs.insert("anthropic".into(), provider);
    cfg
}

// ---------------------------------------------------------------------------
// D-09 case 1: Happy path — ANTHROPIC_AUTH_TOKEN env -> bearer Ok(Some(..., true))
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn auth_token_env_resolves_to_bearer() {
    reset_anthropic_env();
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", "btr-test-1");

    let cfg = Config::default();
    let res = cfg.resolve_anthropic_auth_async().await.unwrap();

    assert_eq!(res, Some(("btr-test-1".to_string(), true)));

    reset_anthropic_env();
}

// ---------------------------------------------------------------------------
// D-09 case 2: Both env vars set → Err naming both vars
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn both_env_vars_set_errors() {
    reset_anthropic_env();
    std::env::set_var("ANTHROPIC_API_KEY", "sk-test-conflict");
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", "btr-test-conflict");

    let cfg = Config::default();
    let err = cfg.resolve_anthropic_auth_async().await.unwrap_err();
    let msg = err.to_string();

    assert!(
        msg.contains("ANTHROPIC_API_KEY") && msg.contains("ANTHROPIC_AUTH_TOKEN"),
        "expected error to name both vars, got: {msg}"
    );

    reset_anthropic_env();
}

// ---------------------------------------------------------------------------
// D-09 case 3: use_bearer_auth=true + ANTHROPIC_API_KEY env → Err
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn pin_bearer_with_env_api_key_errors() {
    reset_anthropic_env();
    std::env::set_var("ANTHROPIC_API_KEY", "sk-test-pinconflict");

    let cfg = anthropic_config_with(ProviderConfig {
        use_bearer_auth: Some(true),
        ..ProviderConfig::default()
    });
    let err = cfg.resolve_anthropic_auth_async().await.unwrap_err();

    assert!(
        err.to_string().contains("use_bearer_auth"),
        "expected error to mention use_bearer_auth, got: {}",
        err
    );

    reset_anthropic_env();
}

// ---------------------------------------------------------------------------
// D-09 case 4: use_bearer_auth=true + provider api_key in settings → Err
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn pin_bearer_with_settings_api_key_errors() {
    reset_anthropic_env();

    let cfg = anthropic_config_with(ProviderConfig {
        api_key: Some("sk-from-settings".into()),
        use_bearer_auth: Some(true),
        ..ProviderConfig::default()
    });
    let err = cfg.resolve_anthropic_auth_async().await.unwrap_err();

    assert!(
        err.to_string().contains("use_bearer_auth"),
        "expected error to mention use_bearer_auth, got: {}",
        err
    );

    reset_anthropic_env();
}

// ---------------------------------------------------------------------------
// D-09 case 5: config.env injection makes ANTHROPIC_AUTH_TOKEN visible → bearer
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn config_env_injection_resolves_bearer() {
    reset_anthropic_env();

    // Simulate the main.rs injection loop (crates/cli depends on crates/core,
    // not the reverse, so we reproduce the loop inline).
    let mut env: HashMap<String, String> = HashMap::new();
    env.insert("ANTHROPIC_AUTH_TOKEN".into(), "btr-from-settings".into());
    for (k, v) in &env {
        if std::env::var(k).is_err() {
            std::env::set_var(k, v);
        }
    }

    let cfg = Config::default();
    let res = cfg.resolve_anthropic_auth_async().await.unwrap();

    assert_eq!(res, Some(("btr-from-settings".to_string(), true)));

    reset_anthropic_env();
}

// ---------------------------------------------------------------------------
// WR-03: use_bearer_auth=true pinned with no token available returns None
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn pin_bearer_with_no_token_returns_none() {
    reset_anthropic_env();
    // No ANTHROPIC_AUTH_TOKEN set, no OAuth tokens on disk
    let cfg = anthropic_config_with(ProviderConfig {
        use_bearer_auth: Some(true),
        ..ProviderConfig::default()
    });
    let res = cfg.resolve_anthropic_auth_async().await.unwrap();
    assert_eq!(res, None);
    reset_anthropic_env();
}

// ---------------------------------------------------------------------------
// WR-04: Injection guard defers to pre-existing env value, not settings value
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn config_env_injection_does_not_overwrite_existing_env() {
    reset_anthropic_env();
    std::env::set_var("ANTHROPIC_AUTH_TOKEN", "btr-from-real-env");

    // Simulate injection with a different value from settings
    let mut env = HashMap::new();
    env.insert("ANTHROPIC_AUTH_TOKEN".into(), "btr-from-settings".into());
    for (k, v) in &env {
        if std::env::var(k).is_err() {
            std::env::set_var(k, v);
        }
    }

    let cfg = Config::default();
    let res = cfg.resolve_anthropic_auth_async().await.unwrap();
    // Real env wins; settings value must not overwrite
    assert_eq!(res, Some(("btr-from-real-env".to_string(), true)));

    reset_anthropic_env();
}

// ---------------------------------------------------------------------------
// Regression: ANTHROPIC_API_KEY alone still resolves to x-api-key (false)
// Guards against Pitfall 2 from RESEARCH.md.
// ---------------------------------------------------------------------------
#[tokio::test]
#[serial]
async fn api_key_only_resolves_to_x_api_key() {
    reset_anthropic_env();
    std::env::set_var("ANTHROPIC_API_KEY", "sk-existing-key");

    let cfg = Config::default();
    let res = cfg.resolve_anthropic_auth_async().await.unwrap();

    assert_eq!(res, Some(("sk-existing-key".to_string(), false)));

    reset_anthropic_env();
}
