use reml_runtime::parse::{ok, run, ParseError, Parser, Reply, Span};
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

fn consume_then_fail(msg: &'static str) -> Parser<()> {
    Parser::new(move |state| {
        let input = state.input().clone();
        if input.is_empty() {
            return Reply::Err {
                error: ParseError::new("入力が足りません", input.position()),
                consumed: false,
                committed: false,
            };
        }
        let _rest = input.advance(1);
        Reply::Err {
            error: ParseError::new(msg, input.position()),
            consumed: true,
            committed: false,
        }
    })
}

fn empty_success() -> Parser<()> {
    Parser::new(|state| {
        let input = state.input().clone();
        let pos = input.position();
        let span = Span::new(pos, pos);
        Reply::Ok {
            value: (),
            span,
            consumed: false,
            rest: input,
        }
    })
}

#[test]
fn or_short_circuits_after_consumed_error() {
    let parser = consume_then_fail("left").or(ok(()));
    let result = run(&parser, "x", &RunConfig::default());
    assert!(
        result.value.is_none(),
        "consumed な左側失敗で右側を試さないはず"
    );
    assert_eq!(result.diagnostics.len(), 1);
}

#[test]
fn attempt_rewinds_consumption_for_alternatives() {
    let attempt_parser = consume_then_fail("try").attempt();
    let fallback = tag("x").map(|_| ());
    let parser = attempt_parser.or(fallback);
    let result = run(&parser, "x", &RunConfig::default());
    assert_eq!(result.value, Some(()));
}

#[test]
fn many_reports_empty_success_body() {
    let parser = empty_success().many();
    let result = run(&parser, "", &RunConfig::default());
    assert!(
        result.value.is_none(),
        "空成功検知時はエラーになるはず"
    );
    assert_eq!(
        result
            .diagnostics
            .first()
            .map(|d| d.message.as_str())
            .unwrap_or(""),
        "繰り返し本体が空成功しました"
    );
}
