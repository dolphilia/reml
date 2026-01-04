//! parser モジュール向けの統合テストセルフ。
//!
//! `TPM-LEX-03` の Packrat メタデータ/期待候補のアラインメント確認のため、
//! `tests/parser/packrat.rs` を含めるエントリポイントです。

#[path = "parser/packrat.rs"]
mod packrat;

#[path = "parser/defer.rs"]
mod defer;

#[path = "parser/qualified_fn_decl.rs"]
mod qualified_fn_decl;
