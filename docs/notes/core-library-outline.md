# 4.1 標準ライブラリ章 骨子（フェーズ2）

## 1. Chapter 4 の位置付けと目的
- Chapter 3 は Core.Parse 以外の共通 API を束ね、Reml の「小さく強いコア」をアプリケーション開発へ拡張する枠組みを提供する。
- フェーズ1 の範囲定義で洗い出した Tier0〜Tier3 のモジュールを、章構成に落とし込みレビュー単位を明確化する。
- Config/Data/Runtime など既存章で定義済みの仕様を Chapter 3 配下に再配置済み。

## 2. 章構成ドラフト（レビュー単位）
| セクション | 想定モジュール | 主な内容 | ステータス |
| --- | --- | --- | --- |
| 3.0 | 範囲定義メモ | 設計ゴール・採否基準・優先度の整理 | ✅ 完了（フェーズ1） |
| 4.1 | 章骨子（本ドキュメント） | モジュール一覧、レビュー単位、索引方針 | ✅ 完了（フェーズ2） |
| 4.2 | Core Prelude & Iteration | `Option`/`Result`、`?` 演算子、`match` 補助、イテレータ／パイプ操作の基本 | ✍️ ドラフト執筆中 |
| 4.3 | Core Collections | 不変リスト／マップ／セット、`Vec`/`Cell` 等の可変構造と効果タグの扱い + 使用例 | ✍️ ドラフト執筆中 |
| 4.4 | Core Text & Unicode | `String`/`Str`/`Bytes`/`Grapheme`、正規化・セグメンテーション、Lex との連携 + 使用例 | ✍️ ドラフト執筆中 |
| 4.5 | Core Numeric & Time | 数値ユーティリティ、`Duration`/`Timestamp`、統計補助 API + 使用例 | ✍️ ドラフト執筆中 |
| 4.6 | Core IO & Path | `io` 効果、`defer` 連携、ファイル／ストリーム／パス操作 + 使用例 | ✍️ ドラフト執筆中 |
| 4.7 | Core Diagnostics & Audit | `Diagnostic` モデル、`audit_id`/`change_set` 共有語彙、CLI/LSP 出力整合 + 使用例 | ✍️ ドラフト執筆中 |
| 4.8 | Core Config & Data | 設定スキーマ／データモデリング章の再配置、差分・監査との連携整理 + 使用例 | ✍️ ドラフト執筆中 |
| 4.9 | Core Runtime & Capability Registry | GC capability、メトリクス API、プラグイン／Capability の統合窓口 + 使用例 | ✍️ ドラフト執筆中 |
| 4.10 | Core Async / FFI / Unsafe（将来拡張） | `Future`/`Task`、`ffi` 効果、`unsafe` 境界、互換性ポリシー（調査メモ） | 🧭 ドラフトメモ更新中 |


## 3. 索引用ハイレベルリンク
- Chapter 2（Core.Parse）から Chapter 4 への参照は、`use Core` 経由での導入例とパーサ以外のユーティリティを対比して整理する。【F:2-1-parser-type.md†L1-L9】
- Config/Data/Runtime 既存章の内容は Chapter 3.7/3.8 に移行済み。元ファイル（2-7〜2-9）は削除済み。
- 横断テーマを扱うガイド（設定 CLI、LSP、Runtime、FFI、プラグイン）は対応する Chapter 4 節への逆リンクを設ける想定で README 索引を更新する。【F:guides/config-cli.md†L1-L7】【F:guides/lsp-integration.md†L1-L6】【F:guides/runtime-bridges.md†L1-L6】【F:guides/reml-ffi-handbook.md†L1-L6】【F:guides/DSL-plugin.md†L1-L6】

## 4. 次ステップ（フェーズ3 への引き継ぎ）
1. Tier 0（3.1〜3.4）について、型定義・主要関数シグネチャ・効果タグの仕様ドラフトを起草する。3.1（Prelude & Iteration）はドラフト補強済みであり、3.2（Collections）と3.3（Text & Unicode）は使用例を含むドラフトへ更新したため、次フェーズでは API 仕様の精緻化とテスト指針の策定を行う。
2. Tier 1（3.4〜3.6）で共有語彙 (`Diagnostic`, `audit_id`, `Duration`, `Path` 等) の共通フォーマットを明文化し、ガイドからの参照を誘導する。
3. Config/Data/Runtime の本文再配置時に差分追跡ルール（リネーム方針、旧リンク対応）を明記するためのドラフトテンプレートを作成する。
4. Async/FFI/Unsafe（3.9）については、効果タグと安全境界の互換性調査メモを用意し、レビュー対象とする範囲を確定する。
5. 2025-11-18 時点で `cargo xtask prelude-audit --wbs 2.1b --strict --baseline docs/spec/3-1-core-prelude-iteration.md` を実行し、`core_prelude_option_result.{rs,snap}` の 16 シナリオ結果と `prelude_api_inventory.toml` の `rust_status=implemented` を `reports/spec-audit/ch0/links.md` に記録した。WBS 2.2 以降の項目は `wbs` フィルタで未実装として追跡を継続する。
