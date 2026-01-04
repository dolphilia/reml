use reml_adapter::env::{
    self as adapter_env, AdapterResult, EnvAuditHandle, EnvAuditor,
    EnvContext as AdapterEnvContext, EnvMutationArgs, EnvMutationResult, EnvOperation, EnvScope,
};
use serde_json::{json, Map, Value};

use crate::audit::{AuditContext, AuditError};

pub use adapter_env::{AdapterError as EnvAdapterError, EnvContext, EnvError, PlatformSnapshot};

/// 環境変数の読み取り（監査なし）。
pub fn get_env(key: &str) -> Result<Option<String>, adapter_env::EnvError> {
    adapter_env::get_env(key, AdapterEnvContext::detect(EnvOperation::Get))
}

/// 指定キーの環境変数を書き換え、監査ログを記録する。
pub fn set_env(
    ctx: &AuditContext,
    key: &str,
    value: &str,
    requested_by: &str,
) -> AdapterResult<()> {
    let args = EnvMutationArgs {
        operation: EnvOperation::Set,
        scope: EnvScope::Process,
        key,
        requested_by,
    };
    let handle = EnvAuditHandle::new(ctx, args);
    adapter_env::set_env(
        key,
        value,
        AdapterEnvContext::detect(args.operation),
        Some(&handle),
    )
}

/// 環境変数の削除と監査。
pub fn remove_env(ctx: &AuditContext, key: &str, requested_by: &str) -> AdapterResult<()> {
    let args = EnvMutationArgs {
        operation: EnvOperation::Remove,
        scope: EnvScope::Process,
        key,
        requested_by,
    };
    let handle = EnvAuditHandle::new(ctx, args);
    adapter_env::remove_env(
        key,
        AdapterEnvContext::detect(args.operation),
        Some(&handle),
    )
}

impl EnvAuditor for AuditContext {
    type AuditError = AuditError;

    fn audit_env_mutation(
        &self,
        args: EnvMutationArgs<'_>,
        result: EnvMutationResult<'_>,
    ) -> Result<(), Self::AuditError> {
        let mut metadata = Map::new();
        metadata.insert(
            "env.operation".into(),
            Value::String(args.operation.as_str().to_string()),
        );
        metadata.insert("env.key".into(), Value::String(args.key.to_string()));
        metadata.insert(
            "env.scope".into(),
            Value::String(args.scope.as_str().to_string()),
        );
        metadata.insert(
            "requested_by".into(),
            Value::String(args.requested_by.to_string()),
        );
        metadata.insert(
            "env.status".into(),
            Value::String(result.status().to_string()),
        );
        if let Some(reason) = result.failure_reason() {
            metadata.insert(
                "env.failure_reason".into(),
                Value::String(reason.to_string()),
            );
        }
        self.clone()
            .with_metadata(metadata)
            .log("env_mutation", json!({}))
    }
}
