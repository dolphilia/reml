use reml_runtime::parse::{
    embedded_dsl, ContextBridge, EmbeddedBoundary, EmbeddedDslSpec, EmbeddedMode, ParseError,
    Parser, Reply,
};
use reml_runtime::run_config::RunConfig;

fn tag(expected: &'static str) -> Parser<&'static str> {
    Parser::new(move |state| {
        let input = state.input().clone();
        if input.remaining().starts_with(expected) {
            let rest = input.advance(expected.len());
            Reply::Ok {
                value: expected,
                span: input.span_to(&rest),
                consumed: !expected.is_empty(),
                rest,
            }
        } else {
            Reply::Err {
                error: ParseError::new("tag で不一致", input.position()),
                consumed: false,
                committed: false,
            }
        }
    })
}

fn embedded_spec<T>(parser: Parser<T>) -> EmbeddedDslSpec<T> {
    EmbeddedDslSpec {
        dsl_id: "reml".to_string(),
        boundary: EmbeddedBoundary::new("<", ">"),
        parser,
        lsp: None,
        mode: EmbeddedMode::SequentialOnly,
        context: ContextBridge::Inherit(vec!["scope".to_string(), "type_env".to_string()]),
    }
}

#[test]
fn embedded_dsl_parses_boundary() {
    let parser = embedded_dsl(embedded_spec(tag("abc")));
    let result = reml_runtime::parse::run(&parser, "<abc>", &RunConfig::default());
    let node = result.value.expect("embedded node should be parsed");
    assert_eq!(node.dsl_id, "reml");
    assert_eq!(node.ast, "abc");
    assert!(node.diagnostics.is_empty());
}

#[test]
fn embedded_dsl_missing_end_reports_error() {
    let parser = embedded_dsl(embedded_spec(tag("abc")));
    let result = reml_runtime::parse::run(&parser, "<abc", &RunConfig::default());
    assert!(result.value.is_none());
    let message = result
        .diagnostics
        .first()
        .map(|diag| diag.message.as_str())
        .unwrap_or("");
    assert_eq!(message, "埋め込み DSL の終了境界が見つかりません");
}

#[test]
fn embedded_dsl_sets_source_dsl_for_child_errors() {
    let parser = embedded_dsl(embedded_spec(tag("abc")));
    let result = reml_runtime::parse::run(&parser, "<x>", &RunConfig::default());
    assert!(result.value.is_none());
    let diag = result.diagnostics.first().expect("diagnostic should exist");
    assert_eq!(diag.source_dsl.as_deref(), Some("reml"));
    assert_eq!(diag.position.line, 1);
    assert_eq!(diag.position.column, 2);
}

#[test]
fn embedded_dsl_empty_input_reports_start_boundary_error() {
    let parser = embedded_dsl(embedded_spec(tag("abc")));
    let result = reml_runtime::parse::run(&parser, "", &RunConfig::default());
    assert!(result.value.is_none());
    let message = result
        .diagnostics
        .first()
        .map(|diag| diag.message.as_str())
        .unwrap_or("");
    assert_eq!(message, "埋め込み DSL の開始境界が見つかりません");
}
