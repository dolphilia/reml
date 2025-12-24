use std::fmt::Debug;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json;

use crate::parse::{run_with_default, ParseResult, Parser};
use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic};

use super::{
    normalize_snapshot, record_snapshot_updated, record_test_diagnostic, snapshot_hash,
    SnapshotMode, SnapshotPolicy, TestError, TestErrorKind, TestResult,
};

#[derive(Clone, Debug)]
pub struct DslCase<T> {
    pub name: Option<String>,
    pub source: String,
    pub expect: DslExpectation<T>,
}

#[derive(Clone, Debug)]
pub enum DslExpectation<T> {
    Ast(AstMatcher<T>),
    Error(ErrorExpectation),
    Golden(GoldenCase),
}

#[derive(Clone, Debug)]
pub enum AstMatcher<T> {
    Exact(T),
    Any,
    Pattern(String),
    List(Vec<AstMatcher<T>>),
    Record(Vec<(String, AstMatcher<T>)>),
}

impl<T: PartialEq + Debug> AstMatcher<T> {
    fn matches(&self, actual: &T) -> bool {
        match self {
            AstMatcher::Exact(expected) => expected == actual,
            AstMatcher::Any => true,
            _ => {
                let actual = normalize_debug(&format!("{actual:#?}"));
                let pattern = matcher_to_pattern_string(self);
                matches_pattern(&actual, &pattern)
            }
        }
    }
}

pub fn pattern<T>(pattern: impl Into<String>) -> AstMatcher<T> {
    AstMatcher::Pattern(pattern.into())
}

pub fn list<T>(items: Vec<AstMatcher<T>>) -> AstMatcher<T> {
    AstMatcher::List(items)
}

pub fn record<T>(fields: Vec<(String, AstMatcher<T>)>) -> AstMatcher<T> {
    AstMatcher::Record(fields)
}

#[derive(Clone, Debug)]
pub struct ErrorExpectation {
    pub code: String,
    pub at: Option<AtSpec>,
    pub message: Option<String>,
}

#[derive(Clone, Debug)]
pub enum AtSpec {
    Offset(usize),
    LineCol { line: usize, col: usize },
}

#[derive(Clone, Debug)]
pub struct GoldenCase {
    pub case_id: String,
    pub input_path: PathBuf,
    pub expected_ast_path: PathBuf,
    pub expected_error_path: PathBuf,
    pub policy: SnapshotPolicy,
}

impl GoldenCase {
    pub fn new(
        case_id: impl Into<String>,
        input_path: impl Into<PathBuf>,
        expected_ast_path: impl Into<PathBuf>,
        expected_error_path: impl Into<PathBuf>,
    ) -> Self {
        Self {
            case_id: case_id.into(),
            input_path: input_path.into(),
            expected_ast_path: expected_ast_path.into(),
            expected_error_path: expected_error_path.into(),
            policy: SnapshotPolicy::verify(),
        }
    }

    pub fn from_case_id(
        case_id: impl Into<String>,
        input_root: impl Into<PathBuf>,
        expected_root: impl Into<PathBuf>,
    ) -> Self {
        let case_id = case_id.into();
        let input_path = input_root.into().join(format!("{case_id}.input"));
        let expected_root = expected_root.into();
        let expected_ast_path = expected_root.join(format!("{case_id}.ast"));
        let expected_error_path = expected_root.join(format!("{case_id}.error"));
        Self {
            case_id,
            input_path,
            expected_ast_path,
            expected_error_path,
            policy: SnapshotPolicy::verify(),
        }
    }
}

pub fn test_parser<T>(parser: Parser<T>, cases: &[DslCase<T>]) -> TestResult
where
    T: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    for (index, case) in cases.iter().enumerate() {
        let case_name = case.name.clone().unwrap_or_else(|| format!("case_{index}"));
        let outcome = match &case.expect {
            DslExpectation::Ast(matcher) => {
                let result = run_with_default(&parser, &case.source);
                match_ast(&result, matcher, &case.source)
            }
            DslExpectation::Error(expectation) => {
                let result = run_with_default(&parser, &case.source);
                match_error(&result, expectation, &case.source)
            }
            DslExpectation::Golden(golden) => match_golden(&parser, golden),
        };
        if let Err(err) = outcome {
            let err = err.with_case_name(case_name);
            record_test_diagnostic(&err);
            return Err(err);
        }
    }
    Ok(())
}

fn match_ast<T>(result: &ParseResult<T>, matcher: &AstMatcher<T>, source: &str) -> TestResult
where
    T: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    if !result.diagnostics.is_empty() {
        return Err(TestError::new(
            TestErrorKind::AssertionFailed,
            "expected AST but diagnostics were emitted",
        )
        .with_context("source", source));
    }
    match result.value.as_ref() {
        Some(value) if matcher.matches(value) => Ok(()),
        Some(value) => Err(TestError::new(
            TestErrorKind::AssertionFailed,
            format!("AST mismatch: actual={value:?}"),
        )
        .with_context("source", source)),
        None => Err(TestError::new(
            TestErrorKind::AssertionFailed,
            "expected AST but parser returned no value",
        )
        .with_context("source", source)),
    }
}

fn match_error<T>(
    result: &ParseResult<T>,
    expectation: &ErrorExpectation,
    source: &str,
) -> TestResult
where
    T: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    let diagnostics: Vec<GuardDiagnostic> = result
        .guard_diagnostics()
        .into_iter()
        .filter(|diag| diag.severity == DiagnosticSeverity::Error)
        .collect();
    if diagnostics.is_empty() {
        return Err(TestError::new(
            TestErrorKind::AssertionFailed,
            "expected error but diagnostics were empty",
        )
        .with_context("source", source));
    }

    let source_len = source.as_bytes().len();
    let candidates = select_preferred_diagnostics(&diagnostics);
    if candidates.iter().any(|diag| {
        matches_error_code(&expectation.code, diag, source_len)
            && matches_error_position(expectation.at.as_ref(), diag)
            && matches_error_message(expectation.message.as_ref(), diag)
    }) {
        Ok(())
    } else {
        let codes = candidates
            .iter()
            .map(|diag| diag.code)
            .collect::<Vec<_>>()
            .join(", ");
        Err(TestError::new(
            TestErrorKind::AssertionFailed,
            format!("error expectation mismatch: expected={}", expectation.code),
        )
        .with_context("actual_codes", codes)
        .with_context("source", source))
    }
}

fn match_golden<T>(parser: &Parser<T>, golden: &GoldenCase) -> TestResult
where
    T: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    let input = read_text(&golden.input_path)?;
    let result = run_with_default(parser, &input);
    let diagnostics = result.guard_diagnostics();
    let error_json = render_error_json(&golden.input_path, &diagnostics);
    let mut updated = false;

    if diagnostics.is_empty() {
        let Some(value) = result.value.as_ref() else {
            return Err(TestError::new(
                TestErrorKind::AssertionFailed,
                "golden case expected value but parser returned none",
            ));
        };
        let ast_text = format!("{value:#?}");
        updated |= apply_golden_snapshot(&golden.expected_ast_path, &ast_text, &golden.policy)?;
        updated |= apply_golden_snapshot(&golden.expected_error_path, &error_json, &golden.policy)?;
        if updated {
            let combined = format!("{ast_text}\n{error_json}");
            record_snapshot_updated(
                &golden.case_id,
                snapshot_hash(&combined),
                golden.policy.mode,
                combined.len(),
            );
        }
        Ok(())
    } else {
        updated |= apply_golden_snapshot(&golden.expected_error_path, &error_json, &golden.policy)?;
        if updated {
            record_snapshot_updated(
                &golden.case_id,
                snapshot_hash(&error_json),
                golden.policy.mode,
                error_json.len(),
            );
        }
        Ok(())
    }
}

fn matches_error_code(expected: &str, diag: &GuardDiagnostic, source_len: usize) -> bool {
    if diagnostic_codes(diag).iter().any(|code| code == expected) {
        return true;
    }
    if expected == "parser.unexpected_eof" && diag.code == "parser.syntax.expected_tokens" {
        return extract_position(diag)
            .map(|(byte, _, _)| byte == source_len)
            .unwrap_or(false);
    }
    false
}

fn matches_error_position(expect: Option<&AtSpec>, diag: &GuardDiagnostic) -> bool {
    let Some(expect) = expect else { return true };
    let Some((byte, line, col)) = extract_position(diag) else {
        return false;
    };
    match expect {
        AtSpec::Offset(offset) => byte == *offset,
        AtSpec::LineCol {
            line: exp_line,
            col: exp_col,
        } => line == *exp_line && col == *exp_col,
    }
}

fn matches_error_message(expect: Option<&String>, diag: &GuardDiagnostic) -> bool {
    let Some(expect) = expect else { return true };
    diag.message.contains(expect)
}

fn extract_position(diag: &GuardDiagnostic) -> Option<(usize, usize, usize)> {
    let position = diag.extensions.get("parser.position")?.as_object()?;
    let byte = position.get("byte")?.as_u64()? as usize;
    let line = position.get("line")?.as_u64()? as usize;
    let column = position.get("column")?.as_u64()? as usize;
    Some((byte, line, column))
}

fn diagnostic_codes(diag: &GuardDiagnostic) -> Vec<String> {
    let mut codes = vec![diag.code.to_string()];
    for key in ["codes", "diagnostic.codes"] {
        if let Some(value) = diag.extensions.get(key) {
            if let Some(items) = value.as_array() {
                for item in items {
                    if let Some(code) = item.as_str() {
                        codes.push(code.to_string());
                    }
                }
            }
        }
    }
    codes.sort();
    codes.dedup();
    codes
}

fn select_preferred_diagnostics(diags: &[GuardDiagnostic]) -> Vec<&GuardDiagnostic> {
    let mut max_byte: Option<usize> = None;
    for diag in diags {
        if let Some((byte, _, _)) = extract_position(diag) {
            max_byte = Some(max_byte.map_or(byte, |current| current.max(byte)));
        }
    }
    if let Some(max_byte) = max_byte {
        diags
            .iter()
            .filter(|diag| {
                extract_position(diag)
                    .map(|(byte, _, _)| byte == max_byte)
                    .unwrap_or(false)
            })
            .collect()
    } else {
        diags.iter().collect()
    }
}

fn read_text(path: &Path) -> Result<String, TestError> {
    fs::read_to_string(path).map_err(|err| {
        TestError::new(
            TestErrorKind::HarnessFailure,
            format!("failed to read golden file: {err}"),
        )
        .with_context("path", path.display().to_string())
    })
}

fn apply_golden_snapshot(
    path: &Path,
    value: &str,
    policy: &SnapshotPolicy,
) -> Result<bool, TestError> {
    let value = if policy.normalize {
        normalize_snapshot(value)
    } else {
        value.to_string()
    };
    match policy.mode {
        SnapshotMode::Verify => {
            let expected = read_text(path)?;
            if expected == value {
                Ok(false)
            } else {
                Err(
                    TestError::new(TestErrorKind::SnapshotMismatch, "golden mismatch")
                        .with_context("path", path.display().to_string()),
                )
            }
        }
        SnapshotMode::Record => {
            if path.exists() {
                let expected = read_text(path)?;
                if expected == value {
                    Ok(false)
                } else {
                    Err(
                        TestError::new(TestErrorKind::SnapshotMismatch, "golden mismatch")
                            .with_context("path", path.display().to_string()),
                    )
                }
            } else {
                write_text(path, &value)?;
                Ok(true)
            }
        }
        SnapshotMode::Update => {
            write_text(path, &value)?;
            Ok(true)
        }
    }
}

fn write_text(path: &Path, value: &str) -> Result<(), TestError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            TestError::new(
                TestErrorKind::HarnessFailure,
                format!("failed to create golden directory: {err}"),
            )
            .with_context("path", parent.display().to_string())
        })?;
    }
    fs::write(path, value).map_err(|err| {
        TestError::new(
            TestErrorKind::HarnessFailure,
            format!("failed to write golden file: {err}"),
        )
        .with_context("path", path.display().to_string())
    })
}

fn render_error_json(path: &Path, diagnostics: &[GuardDiagnostic]) -> String {
    let payload = serde_json::json!({
        "schema_version": "3.0.0-alpha",
        "scenario": path.display().to_string(),
        "diagnostics": diagnostics.iter().cloned().map(GuardDiagnostic::into_json).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&payload).unwrap_or_else(|_| {
        "{\"schema_version\":\"3.0.0-alpha\",\"scenario\":\"\",\"diagnostics\":[]}".to_string()
    })
}

fn normalize_debug(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn matches_pattern(actual: &str, pattern: &str) -> bool {
    if pattern == "..." {
        return true;
    }
    let parts: Vec<&str> = pattern.split("...").collect();
    let mut index = 0usize;
    let mut start = 0usize;

    if !pattern.starts_with("...") {
        if let Some(first) = parts.first() {
            if !actual.starts_with(first) {
                return false;
            }
            index = first.len();
            start = 1;
        }
    }

    let end_anchor = !pattern.ends_with("...");
    let last_index = if end_anchor && parts.len() > start {
        parts.len() - 1
    } else {
        parts.len()
    };

    for part in &parts[start..last_index] {
        if part.is_empty() {
            continue;
        }
        if let Some(found) = actual[index..].find(part) {
            index += found + part.len();
        } else {
            return false;
        }
    }

    if end_anchor {
        if let Some(last) = parts.last() {
            return actual[index..].ends_with(last);
        }
    }
    true
}

fn matcher_to_pattern_string<T: Debug>(matcher: &AstMatcher<T>) -> String {
    match matcher {
        AstMatcher::Exact(value) => normalize_debug(&format!("{value:#?}")),
        AstMatcher::Any => "...".to_string(),
        AstMatcher::Pattern(pattern) => normalize_debug(pattern),
        AstMatcher::List(items) => {
            let rendered = items
                .iter()
                .map(matcher_to_pattern_string)
                .collect::<Vec<_>>()
                .join(", ");
            normalize_debug(&format!("[{rendered}]"))
        }
        AstMatcher::Record(fields) => {
            let rendered = fields
                .iter()
                .map(|(key, value)| format!("{key}: {}", matcher_to_pattern_string(value)))
                .collect::<Vec<_>>()
                .join(", ");
            normalize_debug(&format!("...{{ {rendered} }}..."))
        }
    }
}
