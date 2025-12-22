# 1.0 言語コア仕様 概要

## 概要

言語コア章は Reml のソースコードを解釈するための最小要素を定義し、構文・型・効果・文字モデルを統合して一貫した静的保証を与えることを目的としています。字句から形式文法までの仕様と、型推論や効果安全性の原則を組み合わせることで、パーサーコンビネーター実装やツールチェーンが同じ基盤を共有できるようにします。

## セクションガイド

- [1.1 構文仕様](1-1-syntax.md): UTF-8 前提の字句規則、宣言・式・演算子の構造、`conductor` など DSL 構文と最小 EBNF の位置付けを示します。`let` 宣言が `match` と同等のパターン束縛を受け付け、網羅性チェックで安全性を担保することもここで定義しています。`@intrinsic` のようなネイティブ属性の構文制約もこの章で扱います。
- [1.2 型システムと推論](1-2-types-Inference.md): プリミティブからトレイトまでの型体系と Hindley-Milner 推論、効果行を含む型注釈・エラー方針に加えて、`DslExportSignature` へ Stage/Capability 要件を組み込む `requires_capabilities`・`stage_bounds` の構造を定義します。
- [1.3 効果システムと安全性](1-3-effects-safety.md): 効果分類、純粋性デフォルト、ハンドラ/Capability 連携、効果行の整列規約、`unsafe` ポインタの扱いなど安全性設計を整理します。`effect {native}` の監査境界と `@cfg` 連携もここで定義します。
- [1.4 Unicode 文字モデル](1-4-test-unicode-model.md): Byte/Char/Grapheme の三層モデル、正規化・境界規則・エラーレポート指針を通じて国際化と診断の整合性を確保し、`Core.Parse.State` と `Diagnostic` が `display_width` を共有して列情報を揃える運用規約を定めます。
- [1.5 形式文法（BNF）](1-5-formal-grammar-bnf.md): 章全体で定義した構文要素を EBNF で集約し、実装者・ツール向けのリファレンスを提供します。

Chapter 1 の構文・型仕様は Chapter 3.6 の `Diagnostic`/`AuditEnvelope` と結びついており、Rust Frontend では `examples/core_diagnostics/*.reml` を `tooling/examples/run_examples.sh --suite core_diagnostics --update-golden` で実行して CLI 出力（`CliDiagnosticEnvelope`）と監査ログの整合性を検証します。ここで取得した JSON/NDJSON は `docs/spec/3-6-core-diagnostics-audit.md` §9 のサンプルに反映され、`schema_version = "3.0.0-alpha"`・`structured_hints`・`run_config.lex`・`stream_meta` を含む最新フィールドを Chapter 1 のサンプルでも継続的に生成します。特に `pipeline_branch.reml` では `capability.id=console` / `effect.stage.*` を伴う `effects.contract.stage_mismatch` を再現し、Stage 情報が Chapter 3.8 の Capability Registry と一致することを `reports/spec-audit/ch3/capability_stage-mismatch-20251206.json` の監査ログで確認できるため、Chapter 1 のサンプルが Stage/Audit トレースを欠かさず導出できることを保証します。
