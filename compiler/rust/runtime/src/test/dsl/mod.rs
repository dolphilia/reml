use std::fmt::Debug;

use crate::parse::{run_with_default, ParseResult, Parser};
use crate::prelude::ensure::{DiagnosticSeverity, GuardDiagnostic};

use super::{assert_snapshot_with, record_test_diagnostic, SnapshotPolicy, TestError, TestErrorKind, TestResult};

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
}

impl<T: PartialEq + Debug> AstMatcher<T> {
    fn matches(&self, actual: &T) -> bool {
        match self {
            AstMatcher::Exact(expected) => expected == actual,
            AstMatcher::Any => true,
        }
    }
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
    pub name: String,
    pub value: String,
    pub policy: SnapshotPolicy,
}

impl GoldenCase {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            policy: SnapshotPolicy::verify(),
        }
    }
}

pub fn test_parser<T>(parser: Parser<T>, cases: &[DslCase<T>]) -> TestResult
where
    T: Clone + PartialEq + Debug + Send + Sync + 'static,
{
    for (index, case) in cases.iter().enumerate() {
        let case_name = case
            .name
            .clone()
            .unwrap_or_else(|| format!("case_{index}"));
        let result = run_with_default(&parser, &case.source);
        let outcome = match &case.expect {
            DslExpectation::Ast(matcher) => match_ast(&result, matcher, &case.source),
            DslExpectation::Error(expectation) => {
                match_error(&result, expectation, &case.source)
            }
            DslExpectation::Golden(golden) => match_golden(golden),
        };
        if let Err(err) = outcome {
            let err = err.with_case_name(case_name);
            record_test_diagnostic(&err);
            return Err(err);
        }
    }
    Ok(())
}

fn match_ast<T>(
    result: &ParseResult<T>,
    matcher: &AstMatcher<T>,
    source: &str,
) -> TestResult
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
    if diagnostics.iter().any(|diag| {
        matches_error_code(&expectation.code, diag, source_len)
            && matches_error_position(expectation.at.as_ref(), diag)
            && matches_error_message(expectation.message.as_ref(), diag)
    }) {
        Ok(())
    } else {
        let codes = diagnostics
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

fn match_golden(golden: &GoldenCase) -> TestResult {
    assert_snapshot_with(golden.policy.clone(), &golden.name, &golden.value)
}

fn matches_error_code(expected: &str, diag: &GuardDiagnostic, source_len: usize) -> bool {
    if diag.code == expected {
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
    let Some((byte, line, col)) = extract_position(diag) else { return false };
    match expect {
        AtSpec::Offset(offset) => byte == *offset,
        AtSpec::LineCol { line: exp_line, col: exp_col } => {
            line == *exp_line && col == *exp_col
        }
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
