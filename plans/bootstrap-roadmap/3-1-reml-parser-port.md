# 3.1 Reml Parser 再実装計画

## 目的
- Phase 3 マイルストーン M1 を達成するため、OCaml 実装の Parser を Reml で再実装し、Core.Parse API (`2-7-core-parse-streaming.md`) を備えたセルフホスト用フロントエンドを構築する。
- ストリーミング API とバッチ API を両立させ、プラグイン・DSL が利用できる拡張ポイントを整える。

## スコープ
- **含む**: Core.Parse モジュール実装、ストリーミング制御 (`run_stream`, `FlowController`)、AST 生成、Span 保持、CLI 連携。
- **含まない**: DSL 専用拡張、ユーザー定義演算子の動的登録（必要であれば後続タスクに委譲）。
- **前提**: Phase 2 までに確定した構文・診断仕様が存在し、OCaml 版で参照できる状態であること。

## 作業ブレークダウン
1. **Core.Parse API 実装**: `2-0-parser-api-overview.md` の契約を Reml コードで再現し、ストリーミング処理の状態遷移を定義。
2. **Lexer/Parser ポート**: OCaml 版の構文定義を Reml へ移植し、構文糖の扱いを統一。
3. **ストリーム適合テスト**: `2-7-core-parse-streaming.md` で定義されたテストケースを実装し、部分入力・バックプレッシャを検証。
4. **診断整合**: Span 情報・期待値を Reml 版でも同じフォーマットで生成し、OCaml 版との diff をレポート。
5. **プラグインフック検討**: `4-7-core-parse-plugin.md` を参照し、将来の DSL フックに備えたインタフェースを提供。
6. **移行段取り**: OCaml 版と Reml 版を並行稼働させるためのフラグ（`--frontend=ocaml|reml`）を CLI に追加。

## 成果物と検証
- AST スナップショット比較で両実装の差分が許容範囲内に収束し、差分理由が記録される。
- ストリーミングテストが CI で通過し、性能が Phase 2 のベースライン ±10% 以内であること。
- プラグイン API の設計メモが作成され、`notes/dsl-plugin-roadmap.md` に連携事項が登録される。

## リスクとフォローアップ
- Reml 実装の性能が低下した場合、OCaml 版をフォールバックとして維持し、`0-4-risk-handling.md` に最適化タスクを記録。
- ストリーミング API の設計が固定される前に DSL 要件が追加される場合、`notes/guides-to-spec-integration-plan.md` と再調整。
- 並行稼働期間中はテストを二重に走らせる必要があるため、CI 負荷増大に備える。

## 参考資料
- [3-0-phase3-self-host.md](3-0-phase3-self-host.md)
- [2-7-core-parse-streaming.md](../../2-7-core-parse-streaming.md)
- [4-7-core-parse-plugin.md](../../4-7-core-parse-plugin.md)
- [notes/guides-to-spec-integration-plan.md](../../notes/guides-to-spec-integration-plan.md)
- [notes/dsl-plugin-roadmap.md](../../notes/dsl-plugin-roadmap.md)

