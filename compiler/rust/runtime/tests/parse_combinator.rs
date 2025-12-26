use reml_runtime::parse::{
    cut_here, keyword, layout_token, ok, position, run, symbol, sync_to, ParseError, ParseFixIt,
    Parser, RecoverAction, Reply, Span,
};
use reml_runtime::run_config::RunConfig;
use serde_json::{Map, Value};

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

fn non_consuming_fail(msg: &'static str) -> Parser<()> {
    Parser::new(move |state| {
        let input = state.input().clone();
        Reply::Err {
            error: ParseError::new(msg, input.position()),
            consumed: false,
            committed: false,
        }
    })
}

fn run_keyword(input: &str, kw: &str) -> reml_runtime::parse::ParseResult<()> {
    let parser = keyword(None::<Parser<()>>, kw);
    run(&parser, input, &RunConfig::default())
}

fn committed_fail(msg: &'static str) -> Parser<i32> {
    Parser::new(move |state| {
        let input = state.input().clone();
        Reply::Err {
            error: ParseError::new(msg, input.position()),
            consumed: false,
            committed: true,
        }
    })
}

fn digit() -> Parser<i32> {
    Parser::new(|state| {
        let input = state.input().clone();
        match input.remaining().chars().next() {
            Some(ch) if ch.is_ascii_digit() => {
                let rest = input.advance(ch.len_utf8());
                Reply::Ok {
                    value: ch.to_digit(10).unwrap() as i32,
                    span: input.span_to(&rest),
                    consumed: true,
                    rest,
                }
            }
            _ => Reply::Err {
                error: ParseError::new("digit で不一致", input.position()),
                consumed: false,
                committed: false,
            },
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
fn cut_blocks_fallback_even_when_empty() {
    let parser = non_consuming_fail("hard stop").cut().or(ok(()));
    let result = run(&parser, "", &RunConfig::default());
    assert!(
        result.value.is_none(),
        "commit 付きの失敗は or で巻き取らない"
    );
    assert_eq!(
        result
            .diagnostics
            .first()
            .map(|d| d.message.as_str())
            .unwrap_or(""),
        "hard stop"
    );
}

#[test]
fn cut_here_inserts_zero_width_consumption() {
    let parser = cut_here()
        .skip_r(non_consuming_fail("after cut_here"))
        .or(ok(()));
    let result = run(&parser, "", &RunConfig::default());
    assert!(
        result.value.is_none(),
        "cut_here のゼロ幅消費後の失敗は or で巻き戻らない"
    );
}

#[test]
fn many_reports_empty_success_body() {
    let parser = empty_success().many();
    let result = run(&parser, "", &RunConfig::default());
    assert!(result.value.is_none(), "空成功検知時はエラーになるはず");
    assert_eq!(
        result
            .diagnostics
            .first()
            .map(|d| d.message.as_str())
            .unwrap_or(""),
        "繰り返し本体が空成功しました"
    );
}

#[test]
fn chainl1_is_left_associative() {
    let parser = digit().chainl1(tag("-").map(|_| |l: i32, r: i32| l - r));
    let cfg = RunConfig {
        require_eof: true,
        ..RunConfig::default()
    };
    let result = run(&parser, "6-2-1", &cfg);
    assert_eq!(result.value, Some(3));
}

#[test]
fn chainr1_is_right_associative() {
    let parser = digit().chainr1(tag("^").map(|_| |l: i32, r: i32| l.pow(r as u32)));
    let cfg = RunConfig {
        require_eof: true,
        ..RunConfig::default()
    };
    let result = run(&parser, "2^3^2", &cfg);
    assert_eq!(result.value, Some(512));
}

#[test]
fn keyword_boundary_rejects_emoji_continuations() {
    let cases = [
        "let🚀",
        "let👨‍💻",
        "let\u{200D}",
        "let\u{FE0F}",
    ];
    for input in cases {
        let result = run_keyword(input, "let");
        assert!(
            result.value.is_none(),
            "キーワード境界の失敗を想定しています: input={input}"
        );
        let message = result
            .diagnostics
            .first()
            .map(|diag| diag.message.as_str())
            .unwrap_or("");
        assert!(
            message.contains("キーワード 'let' の後ろに識別子が続いています"),
            "想定メッセージが含まれません: input={input}, message={message}"
        );
    }
}

#[test]
fn keyword_boundary_rejects_bidi_control() {
    let result = run_keyword("let\u{202E}", "let");
    assert!(
        result.value.is_none(),
        "Bidi 制御文字の拒否を想定しています"
    );
    let message = result
        .diagnostics
        .first()
        .map(|diag| diag.message.as_str())
        .unwrap_or("");
    assert!(
        message.contains("Bidi 制御文字"),
        "Bidi 制御文字の診断が含まれません: {message}"
    );
}

#[test]
fn profile_collects_packrat_and_backtrack_metrics() {
    let branch = tag("x");
    let parser = branch.clone().attempt().or(branch.clone());
    let cfg = RunConfig {
        packrat: true,
        profile: true,
        ..RunConfig::default()
    };
    let result = run(&parser, "y", &cfg);
    let profile = result
        .profile
        .as_ref()
        .expect("profile should be collected");
    assert!(
        profile.packrat_hits >= 1,
        "expected at least one memo hit on retry"
    );
    assert!(
        profile.packrat_misses >= 1,
        "initial calls should record memo misses"
    );
    assert_eq!(profile.backtracks, 1, "attempt should record one backtrack");
    assert!(
        profile.memo_entries >= 1,
        "memoized entries should be tracked"
    );
}

#[test]
fn profile_stays_disabled_by_default() {
    let result = run(&ok(()), "", &RunConfig::default());
    assert!(result.profile.is_none(), "profile is opt-in");
}

#[test]
fn recover_is_disabled_by_default() {
    let parser = consume_then_fail("recover me").recover(tag("\n").map(|_| ()), ());
    let result = run(&parser, "oops\nrest", &RunConfig::default());
    assert!(result.value.is_none(), "recover は既定では無効のはず");
    assert!(!result.recovered, "recover 無効時は recovered も立たない");
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].message, "recover me");
}

fn recover_collect_config(
    sync_tokens: &[&'static str],
    max_diagnostics: Option<u64>,
    max_resync_bytes: Option<u64>,
    max_recoveries: Option<u64>,
) -> RunConfig {
    RunConfig::default().with_extension("recover", |mut ext| {
        ext.insert("mode".into(), Value::String("collect".into()));
        if !sync_tokens.is_empty() {
            ext.insert(
                "sync_tokens".into(),
                Value::Array(
                    sync_tokens
                        .iter()
                        .map(|token| Value::String((*token).into()))
                        .collect(),
                ),
            );
        }
        if let Some(value) = max_diagnostics {
            ext.insert("max_diagnostics".into(), Value::from(value));
        }
        if let Some(value) = max_resync_bytes {
            ext.insert("max_resync_bytes".into(), Value::from(value));
        }
        if let Some(value) = max_recoveries {
            ext.insert("max_recoveries".into(), Value::from(value));
        }
        ext
    })
}

#[test]
fn recover_collect_mode_records_sync_token() {
    let cfg = recover_collect_config(&["\n"], None, None, None);
    let parser = consume_then_fail("recover me").recover(tag("\n").map(|_| ()), ());
    let result = run(&parser, "oops\nrest", &cfg);
    assert_eq!(result.value, Some(()));
    assert!(result.recovered, "recover 有効時は recovered が立つはず");
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].message, "recover me");
    assert_eq!(
        result.diagnostics[0]
            .recover
            .as_ref()
            .and_then(|meta| meta.mode.as_deref()),
        Some("collect")
    );
    assert_eq!(
        result.diagnostics[0]
            .recover
            .as_ref()
            .and_then(|meta| meta.action.as_ref()),
        Some(&RecoverAction::Skip)
    );
    assert_eq!(
        result.diagnostics[0]
            .recover
            .as_ref()
            .and_then(|meta| meta.sync.as_deref()),
        Some("\n")
    );
    let span = result.span.expect("recover 成功時は span が付与される");
    assert_eq!(span.start.line, 1);
    assert_eq!(span.end.line, 2);
}

#[test]
fn recover_collect_mode_can_recover_committed_failure_without_trying_fallback() {
    let cfg = recover_collect_config(&["\n"], None, None, None);
    let parser = committed_fail("hard stop")
        .recover(tag("\n").map(|_| ()), 10)
        .or(ok(20));
    let result = run(&parser, "oops\nrest", &cfg);
    assert_eq!(result.value, Some(10));
    assert!(result.recovered, "committed 失敗でも recover できるはず");
    assert_eq!(result.diagnostics.len(), 1);
    assert_eq!(result.diagnostics[0].message, "hard stop");
}

#[test]
fn recover_with_default_records_action_default() {
    let cfg = recover_collect_config(&["\n"], None, None, None);
    let parser = consume_then_fail("recover me").recover_with_default(tag("\n").map(|_| ()), ());
    let result = run(&parser, "oops\nrest", &cfg);
    assert_eq!(result.value, Some(()));
    assert!(result.recovered);
    let meta = result.diagnostics[0]
        .recover
        .as_ref()
        .expect("recover meta should be attached");
    assert_eq!(meta.action.as_ref(), Some(&RecoverAction::Default));
}

#[test]
fn recover_with_insert_records_inserted_and_fixit() {
    let cfg = recover_collect_config(&["\n"], None, None, None);
    let parser =
        consume_then_fail("recover me").recover_with_insert(tag("\n").map(|_| ()), ";", ());
    let result = run(&parser, "oops\nrest", &cfg);
    assert_eq!(result.value, Some(()));
    assert!(result.recovered);
    let meta = result.diagnostics[0]
        .recover
        .as_ref()
        .expect("recover meta should be attached");
    assert_eq!(meta.action.as_ref(), Some(&RecoverAction::Insert));
    assert_eq!(meta.inserted.as_deref(), Some(";"));
    assert_eq!(
        result.diagnostics[0].fixits,
        vec![ParseFixIt::InsertToken { token: ";".into() }]
    );
    let guard = result.diagnostics[0].to_guard_diagnostic();
    assert!(
        guard.extensions.get("fixits").is_some(),
        "to_guard_diagnostic should expose fixits"
    );
}

#[test]
fn recover_with_context_records_context_message() {
    let cfg = recover_collect_config(&["\n"], None, None, None);
    let parser = consume_then_fail("recover me").recover_with_context(
        tag("\n").map(|_| ()),
        "ここは式が必要です",
        (),
    );
    let result = run(&parser, "oops\nrest", &cfg);
    assert_eq!(result.value, Some(()));
    assert!(result.recovered);
    let meta = result.diagnostics[0]
        .recover
        .as_ref()
        .expect("recover meta should be attached");
    assert_eq!(meta.action.as_ref(), Some(&RecoverAction::Context));
    assert_eq!(meta.context.as_deref(), Some("ここは式が必要です"));
}

#[test]
fn sync_to_consumes_sync_token() {
    let parser = sync_to(symbol(None, ";")).skip_l(tag("rest"));
    let result = run(&parser, "oops;rest", &RunConfig::default());
    assert_eq!(result.value, Some("rest"));
}

#[test]
fn panic_block_skips_nested_block() {
    let cfg = recover_collect_config(&["}"], None, None, None);
    let parser = symbol(None, "{")
        .skip_l(non_consuming_fail("panic"))
        .panic_block(symbol(None, "{"), symbol(None, "}"), ())
        .skip_l(tag("tail"));
    let result = run(&parser, "{ { } }tail", &cfg);
    assert_eq!(result.value, Some("tail"));
    assert!(result.recovered);
    assert_eq!(result.diagnostics.len(), 1);
}

#[test]
fn recover_missing_inserts_token_and_fixit() {
    let cfg = recover_collect_config(&["\n"], None, None, None);
    let parser = consume_then_fail("recover me").recover_missing(tag("\n").map(|_| ()), ";", ());
    let result = run(&parser, "oops\nrest", &cfg);
    assert_eq!(result.value, Some(()));
    assert!(result.recovered);
    let meta = result.diagnostics[0]
        .recover
        .as_ref()
        .expect("recover meta should be attached");
    assert_eq!(meta.action.as_ref(), Some(&RecoverAction::Insert));
    assert_eq!(meta.inserted.as_deref(), Some(";"));
    assert_eq!(
        result.diagnostics[0].fixits,
        vec![ParseFixIt::InsertToken { token: ";".into() }]
    );
}

#[test]
fn recover_notes_true_exposes_context_in_notes() {
    let cfg =
        recover_collect_config(&["\n"], None, None, None).with_extension("recover", |mut ext| {
            ext.insert("notes".into(), Value::Bool(true));
            ext
        });
    let parser = consume_then_fail("recover me").recover_with_context(
        tag("\n").map(|_| ()),
        "ここは式が必要です",
        (),
    );
    let result = run(&parser, "oops\nrest", &cfg);
    assert_eq!(result.value, Some(()));
    assert!(result.recovered);
    assert_eq!(
        result.diagnostics[0].notes,
        vec!["ここは式が必要です".to_string()]
    );
    let json = result.diagnostics[0].to_guard_diagnostic().into_json();
    assert_eq!(
        json.get("notes")
            .and_then(|value| value.as_array())
            .and_then(|arr| arr.first())
            .and_then(|value| value.get("message"))
            .and_then(|value| value.as_str()),
        Some("ここは式が必要です")
    );
}

#[test]
fn recover_collect_mode_respects_max_recoveries() {
    let cfg = recover_collect_config(&["\n"], None, None, Some(0));
    let parser = consume_then_fail("recover me").recover(tag("\n").map(|_| ()), ());
    let result = run(&parser, "oops\nrest", &cfg);
    assert!(
        result.value.is_none(),
        "max_recoveries=0 では回復しない（fail-fast）"
    );
    assert!(!result.recovered);
}

#[test]
fn recover_collect_mode_respects_max_resync_bytes() {
    let cfg = recover_collect_config(&["\n"], None, Some(2), None);
    let parser = consume_then_fail("recover me").recover(tag("\n").map(|_| ()), ());
    let result = run(&parser, "oops\nrest", &cfg);
    assert!(
        result.value.is_none(),
        "max_resync_bytes が小さい場合は回復を打ち切る（fail-fast）"
    );
}

#[test]
fn spanned_and_position_report_offsets() {
    let parser = position()
        .then(tag("hi").spanned())
        .then(position())
        .map(|((start, (_, mid)), end)| (start, mid, end));
    let cfg = RunConfig {
        require_eof: true,
        ..RunConfig::default()
    };
    let result = run(&parser, "hi", &cfg);
    let (start, mid, end) = result.value.expect("値が返るはず");
    assert_eq!(start.start.line, 1);
    assert_eq!(start.start.column, 1);
    assert_eq!(mid.start.column, 1);
    assert_eq!(mid.end.column, 3);
    assert_eq!(end.start.column, 3);
}

fn layout_run_config() -> RunConfig {
    RunConfig::default().with_extension("lex", |mut m| {
        let mut layout = Map::new();
        layout.insert("offside".into(), Value::Bool(true));
        m.insert("layout_profile".into(), Value::Object(layout));
        m
    })
}

#[test]
fn layout_tokens_emit_indent_and_dedent() {
    let cfg = layout_run_config();
    let parser = symbol(None, "if")
        .then(layout_token("<newline>"))
        .then(layout_token("<indent>"))
        .then(symbol(None, "x"))
        .then(layout_token("<newline>"))
        .then(symbol(None, "y"))
        .then(layout_token("<newline>"))
        .then(layout_token("<dedent>"));
    let result = run(&parser, "if\n  x\n  y\n", &cfg);
    assert!(
        result.value.is_some(),
        "レイアウト付きで成功するはず: diagnostics={:?}",
        result.diagnostics
    );
    assert!(
        result.diagnostics.is_empty(),
        "レイアウト有効時は診断なしで通る想定"
    );
}

#[test]
fn layout_reports_mixed_indent() {
    let cfg = layout_run_config();
    let parser = symbol(None, "if")
        .then(layout_token("<newline>"))
        .then(layout_token("<indent>"))
        .then(symbol(None, "x"))
        .then(layout_token("<dedent>"));
    let result = run(&parser, "if\n \tx", &cfg);
    assert!(
        result.value.is_some(),
        "混在インデントでもパース自体は継続: diagnostics={:?}",
        result.diagnostics
    );
    assert!(
        result
            .diagnostics
            .iter()
            .any(|d| d.message.contains("混在")),
        "混在インデント診断 lex.layout.* が記録されること"
    );
}
