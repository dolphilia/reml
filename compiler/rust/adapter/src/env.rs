use std::{borrow::Cow, env, panic};

use thiserror::Error;

/// 環境 API で利用する操作種別。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnvOperation {
    Get,
    Set,
    Remove,
}

impl EnvOperation {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnvOperation::Get => "get",
            EnvOperation::Set => "set",
            EnvOperation::Remove => "remove",
        }
    }
}

/// 監査用のスコープ。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnvScope {
    Process,
    Session,
    System,
}

impl EnvScope {
    pub fn as_str(&self) -> &'static str {
        match self {
            EnvScope::Process => "process",
            EnvScope::Session => "session",
            EnvScope::System => "system",
        }
    }
}

/// 実行環境の簡易プラットフォーム情報。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlatformSnapshot {
    pub os: String,
    pub family: String,
    pub arch: String,
    pub triple: Option<String>,
    pub profile_id: Option<String>,
    pub stdlib_version: Option<String>,
    pub runtime_revision: Option<String>,
}

impl PlatformSnapshot {
    /// 実行時に `std::env::consts` から取得できる項目のみを埋める。
    pub fn detect() -> Self {
        let os = env::consts::OS.to_string();
        let family = match os.as_str() {
            "linux" | "macos" | "freebsd" | "openbsd" | "android" | "ios" => "unix",
            "windows" => "windows",
            "wasm" => "wasm",
            _ => "other",
        }
        .to_string();
        Self {
            os,
            family,
            arch: env::consts::ARCH.to_string(),
            triple: None,
            profile_id: None,
            stdlib_version: None,
            runtime_revision: None,
        }
    }
}

/// `EnvError` に紐づく実行状況。
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnvContext {
    pub operation: EnvOperation,
    pub platform: PlatformSnapshot,
}

impl EnvContext {
    /// 現在の実行環境を使って `EnvContext` を構築する。
    pub fn detect(operation: EnvOperation) -> Self {
        Self {
            operation,
            platform: PlatformSnapshot::detect(),
        }
    }

    /// 明示的な `PlatformSnapshot` を割り当てる。
    pub fn with_platform(mut self, platform: PlatformSnapshot) -> Self {
        self.platform = platform;
        self
    }
}

/// 環境操作の失敗を表すエラー。
#[derive(Clone, Debug, Error, PartialEq, Eq)]
#[error("{kind:?}: {message}")]
pub struct EnvError {
    pub kind: EnvErrorKind,
    pub message: String,
    pub key: Option<String>,
    pub context: Option<EnvContext>,
}

/// `EnvError` の分類。
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum EnvErrorKind {
    NotFound,
    PermissionDenied,
    InvalidEncoding,
    UnsupportedPlatform,
    IoFailure,
}

/// アダプタ層に共通する失敗。
#[derive(Debug, Error)]
pub enum AdapterError {
    #[error(transparent)]
    Env(#[from] EnvError),
    #[error("audit failure: {0}")]
    Audit(#[source] Box<dyn std::error::Error + Send + Sync>),
}

pub type AdapterResult<T> = Result<T, AdapterError>;

/// 監査に渡すメタ情報。
#[derive(Copy, Clone, Debug)]
pub struct EnvMutationArgs<'a> {
    pub operation: EnvOperation,
    pub scope: EnvScope,
    pub key: &'a str,
    pub requested_by: &'a str,
}

/// `EnvAuditor` が補足できるイベント結果。
#[derive(Clone, Debug)]
pub enum EnvMutationResult<'a> {
    Success,
    Failure(Cow<'a, str>),
}

impl<'a> EnvMutationResult<'a> {
    /// 監査メタとして書き出すステータス。
    pub fn status(&self) -> &'static str {
        match self {
            EnvMutationResult::Success => "success",
            EnvMutationResult::Failure(_) => "failure",
        }
    }

    pub fn failure_reason(&self) -> Option<&str> {
        if let EnvMutationResult::Failure(reason) = self {
            Some(reason.as_ref())
        } else {
            None
        }
    }
}

/// `AuditContext` 等に実装される監査トレイト。
pub trait EnvAuditor {
    type AuditError: std::error::Error + Send + Sync + 'static;

    fn audit_env_mutation(
        &self,
        args: EnvMutationArgs<'_>,
        result: EnvMutationResult<'_>,
    ) -> Result<(), Self::AuditError>;
}

/// 監査情報と一緒に `set_env`/`remove_env` を呼び出すためのハンドル。
#[derive(Copy, Clone, Debug)]
pub struct EnvAuditHandle<'a, A>
where
    A: EnvAuditor,
{
    pub auditor: &'a A,
    pub args: EnvMutationArgs<'a>,
}

impl<'a, A> EnvAuditHandle<'a, A>
where
    A: EnvAuditor,
{
    pub fn new(auditor: &'a A, args: EnvMutationArgs<'a>) -> Self {
        Self { auditor, args }
    }
}

/// 環境変数の読み取り。失敗時は `EnvError` を返す。
pub fn get_env(key: &str, context: EnvContext) -> Result<Option<String>, EnvError> {
    ensure_valid_key(key, &context)?;
    match env::var(key) {
        Ok(value) => Ok(Some(value)),
        Err(env::VarError::NotPresent) => Ok(None),
        Err(env::VarError::NotUnicode(_)) => Err(build_env_error(
            EnvErrorKind::InvalidEncoding,
            "環境変数の値が UTF-8 ではありません",
            Some(key.to_string()),
            &context,
        )),
    }
}

/// 環境変数の設定。監査ハンドルが与えられていれば `EnvMutation` を通知する。
pub fn set_env<A>(
    key: &str,
    value: &str,
    context: EnvContext,
    audit: Option<&EnvAuditHandle<'_, A>>,
) -> AdapterResult<()>
where
    A: EnvAuditor,
{
    if let Err(err) = ensure_valid_key(key, &context) {
        log_result(
            audit,
            EnvMutationResult::Failure(Cow::Owned(err.message.clone())),
        )?;
        return Err(err.into());
    }
    if let Err(err) = ensure_valid_value(value, &context) {
        log_result(
            audit,
            EnvMutationResult::Failure(Cow::Owned(err.message.clone())),
        )?;
        return Err(err.into());
    }

    if let Err(panic_err) = panic::catch_unwind(|| env::set_var(key, value)) {
        let message = format!("set_var が panic しました: {:?}", panic_err);
        log_result(
            audit,
            EnvMutationResult::Failure(Cow::Owned(message.clone())),
        )?;
        return Err(EnvError {
            kind: EnvErrorKind::IoFailure,
            message,
            key: Some(key.to_string()),
            context: Some(context.clone()),
        }
        .into());
    }

    log_result(audit, EnvMutationResult::Success)?;
    Ok(())
}

/// 環境変数の削除。存在しなくても成功と見なす。
pub fn remove_env<A>(
    key: &str,
    context: EnvContext,
    audit: Option<&EnvAuditHandle<'_, A>>,
) -> AdapterResult<()>
where
    A: EnvAuditor,
{
    if let Err(err) = ensure_valid_key(key, &context) {
        log_result(
            audit,
            EnvMutationResult::Failure(Cow::Owned(err.message.clone())),
        )?;
        return Err(err.into());
    }

    if let Err(panic_err) = panic::catch_unwind(|| env::remove_var(key)) {
        let message = format!("remove_var が panic しました: {:?}", panic_err);
        log_result(
            audit,
            EnvMutationResult::Failure(Cow::Owned(message.clone())),
        )?;
        return Err(EnvError {
            kind: EnvErrorKind::IoFailure,
            message,
            key: Some(key.to_string()),
            context: Some(context.clone()),
        }
        .into());
    }

    log_result(audit, EnvMutationResult::Success)?;
    Ok(())
}

fn log_result<A>(
    audit: Option<&EnvAuditHandle<'_, A>>,
    result: EnvMutationResult<'_>,
) -> AdapterResult<()>
where
    A: EnvAuditor,
{
    if let Some(handle) = audit {
        handle
            .auditor
            .audit_env_mutation(handle.args, result)
            .map_err(|err| AdapterError::Audit(Box::new(err)))?;
    }
    Ok(())
}

fn ensure_valid_key(key: &str, context: &EnvContext) -> Result<(), EnvError> {
    if key.is_empty() {
        return Err(build_env_error(
            EnvErrorKind::InvalidEncoding,
            "キーは空にできません",
            None,
            context,
        ));
    }
    if key.contains('\0') {
        return Err(build_env_error(
            EnvErrorKind::InvalidEncoding,
            "キーに NULL バイトを含めることはできません",
            Some(key.to_string()),
            context,
        ));
    }
    Ok(())
}

fn ensure_valid_value(value: &str, context: &EnvContext) -> Result<(), EnvError> {
    if value.contains('\0') {
        return Err(build_env_error(
            EnvErrorKind::InvalidEncoding,
            "値に NULL バイトを含めることはできません",
            None,
            context,
        ));
    }
    Ok(())
}

fn build_env_error(
    kind: EnvErrorKind,
    message: impl Into<String>,
    key: Option<String>,
    context: &EnvContext,
) -> EnvError {
    EnvError {
        kind,
        message: message.into(),
        key,
        context: Some(context.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;

    struct RecordingAuditor {
        events: RefCell<Vec<Recorded>>,
    }

    struct Recorded {
        key: String,
        status: String,
    }

    impl RecordingAuditor {
        fn new() -> Self {
            Self {
                events: RefCell::new(Vec::new()),
            }
        }

        fn take(&self) -> Vec<Recorded> {
            self.events.replace(Vec::new())
        }
    }

    impl EnvAuditor for RecordingAuditor {
        type AuditError = std::convert::Infallible;

        fn audit_env_mutation(
            &self,
            args: EnvMutationArgs<'_>,
            result: EnvMutationResult<'_>,
        ) -> Result<(), Self::AuditError> {
            let status = match result {
                EnvMutationResult::Success => "success".to_string(),
                EnvMutationResult::Failure(message) => message.into_owned(),
            };
            self.events.borrow_mut().push(Recorded {
                key: args.key.to_string(),
                status,
            });
            Ok(())
        }
    }

    fn env_handle<'a, A: EnvAuditor>(
        auditor: &'a A,
        args: EnvMutationArgs<'a>,
    ) -> EnvAuditHandle<'a, A> {
        EnvAuditHandle::new(auditor, args)
    }

    #[test]
    fn set_env_audits_success() {
        let auditor = RecordingAuditor::new();
        let handle = env_handle(
            &auditor,
            EnvMutationArgs {
                operation: EnvOperation::Set,
                scope: EnvScope::Process,
                key: "REML_ADAPTER_TEST",
                requested_by: "test",
            },
        );
        let ctx = EnvContext::detect(EnvOperation::Set);
        set_env(handle.args.key, "value", ctx, Some(&handle)).unwrap();
        let events = auditor.take();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].status, "success");
        assert_eq!(events[0].key, "REML_ADAPTER_TEST");
        std::env::remove_var("REML_ADAPTER_TEST");
    }

    #[test]
    fn set_env_invalid_key_logs_failure() {
        let auditor = RecordingAuditor::new();
        let handle = env_handle(
            &auditor,
            EnvMutationArgs {
                operation: EnvOperation::Set,
                scope: EnvScope::Process,
                key: "has\0null",
                requested_by: "test",
            },
        );
        let ctx = EnvContext::detect(EnvOperation::Set);
        let result = set_env(handle.args.key, "value", ctx, Some(&handle));
        assert!(result.is_err());
        let events = auditor.take();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0].status,
            "キーに NULL バイトを含めることはできません"
        );
    }
}
