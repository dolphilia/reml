use std::error::Error;
use std::fmt;
use std::fs;
use std::path::{Component, Path as StdPath, PathBuf as StdPathBuf};

use serde_json::{Map, Value};

use crate::io::FsAdapter;
use crate::prelude::{
    ensure::{DiagnosticSeverity, GuardDiagnostic, IntoDiagnostic},
    iter::{EffectLabels, EffectSet},
};

use super::{encode_effect_labels, normalize_components, PathBuf};

const CAPABILITY_SECURITY_POLICY: &str = "security.fs.policy";
const CAPABILITY_SYMLINK_QUERY: &str = "fs.symlink.query";

/// Security ヘルパの結果型。
pub type PathSecurityResult<T> = Result<T, PathSecurityError>;

/// パス検証時に参照するポリシー。
#[derive(Debug, Clone)]
pub struct SecurityPolicy {
    allowed_roots: Vec<PathBuf>,
    allow_relative: bool,
    allow_symlinks: bool,
    name: Option<String>,
    digest: Option<String>,
}

impl Default for SecurityPolicy {
    fn default() -> Self {
        Self {
            allowed_roots: Vec::new(),
            allow_relative: false,
            allow_symlinks: false,
            name: None,
            digest: None,
        }
    }
}

impl SecurityPolicy {
    /// 新しいポリシーを生成する。
    pub fn new() -> Self {
        Self::default()
    }

    /// ポリシー名を設定する。
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    /// ポリシーダイジェストを設定する。
    pub fn with_digest(mut self, digest: impl Into<String>) -> Self {
        self.digest = Some(digest.into());
        self
    }

    /// 許可ルートを追加する。
    pub fn add_allowed_root(mut self, root: PathBuf) -> Self {
        self.allowed_roots.push(root.normalize());
        self
    }

    /// 相対パスを許可するか設定する。
    pub fn allow_relative(mut self, allow: bool) -> Self {
        self.allow_relative = allow;
        self
    }

    /// シンボリックリンク転送を許可するか設定する。
    pub fn allow_symlinks(mut self, allow: bool) -> Self {
        self.allow_symlinks = allow;
        self
    }

    pub fn allowed_roots(&self) -> &[PathBuf] {
        &self.allowed_roots
    }

    pub fn allows_relative(&self) -> bool {
        self.allow_relative
    }

    pub fn allows_symlinks(&self) -> bool {
        self.allow_symlinks
    }

    pub fn policy_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn policy_digest(&self) -> Option<&str> {
        self.digest.as_deref()
    }
}

/// パスセキュリティエラー。
#[derive(Debug, Clone)]
pub struct PathSecurityError {
    kind: PathSecurityErrorKind,
    reason: SecurityViolationReason,
    message: String,
    offending_path: Option<PathBuf>,
    normalized_path: Option<PathBuf>,
    sandbox_root: Option<PathBuf>,
    policy_name: Option<String>,
    policy_digest: Option<String>,
    capability: Option<&'static str>,
    effects: EffectLabels,
}

/// エラー種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathSecurityErrorKind {
    InvalidInput,
    SandboxViolation,
    SymlinkViolation,
    CapabilityDenied,
    Io,
}

/// セキュリティ理由。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityViolationReason {
    RelativePathDenied,
    TraversalDetected,
    OutsideAllowedRoot,
    CapabilityDenied,
    SymlinkAbsoluteTarget,
    SymlinkTraversal,
    SymlinkInspectionFailed,
    IoError,
}

impl PathSecurityError {
    /// エラー種別を返す。
    pub fn kind(&self) -> PathSecurityErrorKind {
        self.kind
    }

    fn new(
        kind: PathSecurityErrorKind,
        reason: SecurityViolationReason,
        message: impl Into<String>,
    ) -> Self {
        Self {
            kind,
            reason,
            message: message.into(),
            offending_path: None,
            normalized_path: None,
            sandbox_root: None,
            policy_name: None,
            policy_digest: None,
            capability: None,
            effects: security_effect_labels(),
        }
    }

    fn with_path(mut self, path: PathBuf) -> Self {
        self.offending_path = Some(path);
        self
    }

    fn with_normalized(mut self, path: PathBuf) -> Self {
        self.normalized_path = Some(path);
        self
    }

    fn with_root(mut self, root: PathBuf) -> Self {
        self.sandbox_root = Some(root);
        self
    }

    fn with_policy(mut self, policy: &SecurityPolicy) -> Self {
        if let Some(name) = policy.policy_name() {
            self.policy_name = Some(name.to_string());
        }
        if let Some(digest) = policy.policy_digest() {
            self.policy_digest = Some(digest.to_string());
        }
        self
    }

    fn with_capability(mut self, capability: &'static str) -> Self {
        self.capability = Some(capability);
        self
    }

    fn capability_denied(
        capability: &'static str,
        detail: impl Into<String>,
        policy: Option<&SecurityPolicy>,
    ) -> Self {
        let err = PathSecurityError::new(
            PathSecurityErrorKind::CapabilityDenied,
            SecurityViolationReason::CapabilityDenied,
            detail,
        )
        .with_capability(capability);
        if let Some(policy) = policy {
            err.with_policy(policy)
        } else {
            err
        }
    }

    fn code(&self) -> &'static str {
        match self.kind {
            PathSecurityErrorKind::InvalidInput => "core.path.security.invalid",
            PathSecurityErrorKind::SandboxViolation => "core.path.security.violation",
            PathSecurityErrorKind::SymlinkViolation => "core.path.security.symlink",
            PathSecurityErrorKind::CapabilityDenied => "core.path.security.capability_denied",
            PathSecurityErrorKind::Io => "core.path.security.io_error",
        }
    }

    fn path_string(path: &PathBuf) -> String {
        path.to_string_lossy().into_owned()
    }
}

impl fmt::Display for PathSecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code(), self.message)
    }
}

impl Error for PathSecurityError {}

impl IntoDiagnostic for PathSecurityError {
    fn into_diagnostic(self) -> GuardDiagnostic {
        let code = self.code();
        let PathSecurityError {
            kind: _,
            reason,
            message,
            offending_path,
            normalized_path,
            sandbox_root,
            policy_name,
            policy_digest,
            capability,
            effects,
        } = self;

        let mut security_meta = Map::new();
        if let Some(path) = offending_path.as_ref() {
            security_meta.insert(
                "path".into(),
                Value::String(PathSecurityError::path_string(path)),
            );
        }
        if let Some(path) = normalized_path.as_ref() {
            security_meta.insert(
                "normalized".into(),
                Value::String(PathSecurityError::path_string(path)),
            );
        }
        if let Some(root) = sandbox_root.as_ref() {
            security_meta.insert(
                "sandbox_root".into(),
                Value::String(PathSecurityError::path_string(root)),
            );
        }
        security_meta.insert("reason".into(), Value::String(reason.as_str().to_string()));
        if let Some(name) = policy_name.as_ref() {
            security_meta.insert("policy".into(), Value::String(name.clone()));
        }
        if let Some(digest) = policy_digest.as_ref() {
            security_meta.insert("policy_digest".into(), Value::String(digest.clone()));
        }
        if let Some(cap) = capability.as_ref() {
            security_meta.insert("capability".into(), Value::String(cap.to_string()));
        }

        let mut extensions = Map::new();
        extensions.insert("security".into(), Value::Object(security_meta.clone()));
        extensions.insert("message".into(), Value::String(message.clone()));
        extensions.insert(
            "effects".into(),
            Value::Object(encode_effect_labels(effects)),
        );

        let mut audit_metadata = Map::new();
        if let Some(path) = offending_path.as_ref() {
            audit_metadata.insert(
                "security.path".into(),
                Value::String(PathSecurityError::path_string(path)),
            );
        }
        if let Some(root) = sandbox_root.as_ref() {
            audit_metadata.insert(
                "security.root".into(),
                Value::String(PathSecurityError::path_string(root)),
            );
        }
        audit_metadata.insert(
            "security.reason".into(),
            Value::String(reason.as_str().to_string()),
        );
        if let Some(name) = policy_name {
            audit_metadata.insert("security.policy".into(), Value::String(name));
        }
        if let Some(digest) = policy_digest {
            audit_metadata.insert("security.policy_digest".into(), Value::String(digest));
        }
        if let Some(cap) = capability {
            audit_metadata.insert("security.capability".into(), Value::String(cap.into()));
        }

        GuardDiagnostic {
            code,
            domain: "runtime",
            severity: DiagnosticSeverity::Error,
            message,
            notes: Vec::new(),
            extensions,
            audit_metadata,
        }
    }
}

impl SecurityViolationReason {
    fn as_str(&self) -> &'static str {
        match self {
            SecurityViolationReason::RelativePathDenied => "relative_path_denied",
            SecurityViolationReason::TraversalDetected => "traversal_detected",
            SecurityViolationReason::OutsideAllowedRoot => "outside_allowed_root",
            SecurityViolationReason::CapabilityDenied => "capability_denied",
            SecurityViolationReason::SymlinkAbsoluteTarget => "symlink_absolute_target",
            SecurityViolationReason::SymlinkTraversal => "symlink_traversal",
            SecurityViolationReason::SymlinkInspectionFailed => "symlink_inspection_failed",
            SecurityViolationReason::IoError => "io_error",
        }
    }
}

/// `validate_path` の実装。
pub fn validate_path(path: &PathBuf, policy: &SecurityPolicy) -> PathSecurityResult<PathBuf> {
    ensure_security_capability(Some(policy))?;
    let normalized = path.normalize();

    if has_parent_components(normalized.as_std_path()) {
        return Err(PathSecurityError::new(
            PathSecurityErrorKind::SandboxViolation,
            SecurityViolationReason::TraversalDetected,
            "path contains traversal components",
        )
        .with_path(path.clone())
        .with_normalized(normalized));
    }

    if !policy.allows_relative() && !normalized.is_absolute() {
        return Err(PathSecurityError::new(
            PathSecurityErrorKind::InvalidInput,
            SecurityViolationReason::RelativePathDenied,
            "relative paths are not permitted by the active security policy",
        )
        .with_path(path.clone())
        .with_normalized(normalized)
        .with_policy(policy));
    }

    if !policy.allowed_roots().is_empty() && normalized.is_absolute() {
        let found = policy
            .allowed_roots()
            .iter()
            .any(|root| path_within_root(&normalized, root));
        if !found {
            return Err(PathSecurityError::new(
                PathSecurityErrorKind::SandboxViolation,
                SecurityViolationReason::OutsideAllowedRoot,
                "path is outside of the allowed roots",
            )
            .with_path(path.clone())
            .with_normalized(normalized)
            .with_policy(policy));
        }
    } else if !policy.allowed_roots().is_empty() && !normalized.is_absolute() {
        return Err(PathSecurityError::new(
            PathSecurityErrorKind::SandboxViolation,
            SecurityViolationReason::RelativePathDenied,
            "policy enforces absolute paths within allowed roots",
        )
        .with_path(path.clone())
        .with_normalized(normalized)
        .with_policy(policy));
    }

    Ok(normalized)
}

/// `sandbox_path` の実装。
pub fn sandbox_path(path: &PathBuf, root: &PathBuf) -> PathSecurityResult<PathBuf> {
    ensure_security_capability(None)?;
    let normalized_root = root.normalize();
    if !normalized_root.is_absolute() {
        return Err(PathSecurityError::new(
            PathSecurityErrorKind::InvalidInput,
            SecurityViolationReason::TraversalDetected,
            "sandbox root must be an absolute path",
        )
        .with_root(normalized_root));
    }

    let normalized_path = path.normalize();
    if has_parent_components(normalized_path.as_std_path()) {
        return Err(PathSecurityError::new(
            PathSecurityErrorKind::SandboxViolation,
            SecurityViolationReason::TraversalDetected,
            "path attempts to traverse outside sandbox",
        )
        .with_path(path.clone())
        .with_normalized(normalized_path)
        .with_root(normalized_root));
    }

    let resolved = if normalized_path.is_absolute() {
        normalized_path
    } else {
        normalized_root
            .join(normalized_path.to_string_lossy())
            .map_err(|err| {
                PathSecurityError::new(
                    PathSecurityErrorKind::InvalidInput,
                    SecurityViolationReason::RelativePathDenied,
                    err.to_string(),
                )
                .with_path(path.clone())
                .with_root(normalized_root.clone())
            })?
    };

    if !path_within_root(&resolved, &normalized_root) {
        return Err(PathSecurityError::new(
            PathSecurityErrorKind::SandboxViolation,
            SecurityViolationReason::OutsideAllowedRoot,
            "resolved path escapes sandbox root",
        )
        .with_path(path.clone())
        .with_normalized(resolved)
        .with_root(normalized_root));
    }

    Ok(resolved)
}

/// `is_safe_symlink` の実装。
pub fn is_safe_symlink(path: &PathBuf) -> PathSecurityResult<bool> {
    ensure_security_capability(None)?;
    FsAdapter::global().ensure_symlink_query().map_err(|err| {
        PathSecurityError::capability_denied(
            CAPABILITY_SYMLINK_QUERY,
            err.message().to_string(),
            None,
        )
    })?;

    let std_path = path.as_std_path();
    let metadata = fs::symlink_metadata(std_path).map_err(|io_err| {
        PathSecurityError::new(
            PathSecurityErrorKind::Io,
            SecurityViolationReason::IoError,
            io_err.to_string(),
        )
        .with_path(path.clone())
    })?;

    if !metadata.file_type().is_symlink() {
        return Ok(false);
    }

    let target = fs::read_link(std_path).map_err(|io_err| {
        PathSecurityError::new(
            PathSecurityErrorKind::SymlinkViolation,
            SecurityViolationReason::SymlinkInspectionFailed,
            io_err.to_string(),
        )
        .with_path(path.clone())
    })?;

    if target.is_absolute() {
        return Err(PathSecurityError::new(
            PathSecurityErrorKind::SymlinkViolation,
            SecurityViolationReason::SymlinkAbsoluteTarget,
            "symlink target is absolute",
        )
        .with_path(path.clone())
        .with_normalized(PathBuf::from_std(target)));
    }

    let normalized_target = PathBuf::from_std(normalize_components(&target));
    if has_parent_components(normalized_target.as_std_path()) {
        return Err(PathSecurityError::new(
            PathSecurityErrorKind::SymlinkViolation,
            SecurityViolationReason::SymlinkTraversal,
            "symlink target traverses outside its parent",
        )
        .with_path(path.clone())
        .with_normalized(normalized_target));
    }

    Ok(true)
}

fn ensure_security_capability(policy: Option<&SecurityPolicy>) -> PathSecurityResult<()> {
    FsAdapter::global().ensure_security_policy().map_err(|err| {
        PathSecurityError::capability_denied(
            CAPABILITY_SECURITY_POLICY,
            format!("security capability unavailable: {}", err),
            policy,
        )
    })
}

fn path_within_root(target: &PathBuf, root: &PathBuf) -> bool {
    target.as_std_path().starts_with(root.as_std_path())
}

fn has_parent_components(path: &StdPath) -> bool {
    path.components()
        .any(|component| matches!(component, Component::ParentDir))
}

fn security_effect_labels() -> EffectLabels {
    let mut set = EffectSet::PURE;
    set.mark_security();
    set.to_labels()
}

impl From<StdPathBuf> for PathBuf {
    fn from(value: StdPathBuf) -> Self {
        PathBuf::from_std(value)
    }
}
