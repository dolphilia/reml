use reml_frontend::parser::ast::Module;
use reml_frontend::parser::{
    LeftRecursionMode, ParseResult, ParserDriver, ParserOptions, RunConfig,
};

const PACKRAT_ERROR_SOURCE: &str = "fn missing(x, y = x + y";

fn run_packrat_parse(source: &str) -> ParseResult<Module> {
    let mut run_config = RunConfig::default();
    run_config.packrat = true;
    run_config.trace = true;
    run_config.merge_warnings = true;
    run_config.require_eof = false;
    run_config.left_recursion = LeftRecursionMode::Auto;
    run_config.legacy_result = true;
    let options = ParserOptions::from_run_config(&run_config);
    ParserDriver::parse_with_options_and_run_config(source, options, run_config.clone())
}

/// Packrat 統計と診断が想定どおり取得できることを確認する。
#[test]
fn packrat_stats_collects_queries_and_hits() {
    let result = run_packrat_parse(PACKRAT_ERROR_SOURCE);
    let stats = result.packrat_stats;
    assert!(stats.queries > 0, "Packrat クエリが 0 だった");
    assert!(
        stats.hits <= stats.queries,
        "ヒット数がクエリ数を超えていた"
    );
    assert!(
        result
            .packrat_cache
            .as_ref()
            .map_or(false, |cache| !cache.is_empty()),
        "Packrat キャッシュが空でした"
    );
    assert!(
        !result.span_trace.is_empty(),
        "span_trace が収集されなかった"
    );
}

/// 診断に期待候補のサマリが含まれていることを検証する。
#[test]
fn diagnostics_include_expected_summary_alternatives() {
    let result = run_packrat_parse(PACKRAT_ERROR_SOURCE);
    assert!(!result.diagnostics.is_empty(), "診断が生成されませんでした");
    let diag = &result.diagnostics[0];
    let summary = diag
        .expected_summary
        .as_ref()
        .expect("expected_summary がありません");
    assert!(
        !summary.alternatives.is_empty(),
        "期待候補サマリに代替リストが含まれていませんでした"
    );
}
