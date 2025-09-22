# 4.0 標準ライブラリ仕様: 範囲定義メモ（フェーズ1）

## 1. 設計ゴールと横断テーマの再確認
- **小さく強いコア × 実用性能**: 末尾最適化や Packrat を必要時のみ有効化し、ゼロコスト抽象を重視するという方針を尊重する。【F:0-1-overview.md†L5-L16】
- **読みやすさ・宣言的スタイル**: 左→右パイプ、名前付き引数、推論の強さを損なわず DSL を組み上げられる API を優先する。【F:0-1-overview.md†L13-L26】
- **高品質診断と監査連携**: 共通の JSON メタデータや `Diagnostic` モデルで CLI/LSP/監査ログを横断する方針と整合させる。【F:0-1-overview.md†L19-L24】
- **Unicode 前提**: `byte/char/grapheme` の三層モデルを前提とした文字列 API を標準化する。【F:0-1-overview.md†L27-L40】
- **横断テーマの統合**: 型安全な設定、ツール連携、プラグイン拡張を支える Core モジュール群と整合する。【F:0-1-overview.md†L42-L53】

## 2. 標準 API がカバーすべき領域
| ドメイン | Reml における主な責務 | 近縁言語の参考点 |
| --- | --- | --- |
| 失敗制御・宣言的基礎 | `Result`/`Option` と `?` 演算子、パターン補助で例外無し設計を支える | Rust `std::prelude`, OCaml/F# `Stdlib` |
| データ構造と反復 | 不変構造＋可変 `Vec`/`Cell`、イテレータ合成、パイプ連携 | Rust `Vec`/`Iterator`, F# `Seq` |
| テキスト・Unicode | `String`/`Str`/`Bytes`/`Grapheme`、正規化・セグメンテーション、Lex 連携 | Rust `std::string`+`unicode-segmentation`, F# `Text` |
| 数値・時刻 | `Duration`/`Timestamp`、統計値・百分位、データ品質 API との整合 | Rust `std::time`, OCaml `Stdlib.Bigarray` |
| IO とリソース | `io` 効果、`defer` 解放保証、ファイル/ストリーム/パス操作 | Rust `std::io`/`std::fs`, OCaml `Stdlib.open_*` |
| 並行・非同期 | `Future`/`Task` 型、スケジューラ設定、`io.async` 属性連携 | Rust `std::future`, F# `Async` |
| 設定 DSL | スキーマ定義、差分適用、CLI 連携、監査証跡 | Rust `serde`+`config`, OCaml `Cmdliner` |
| データモデリング | スキーマ/列統計/マイグレーション DSL、品質検証 | F# Type Provider, Rust `polars` |
| 診断と監査 | `Diagnostic` モデル、`audit_id`/`change_set`、CLI/LSP 共通整形 | Rust `miette`, F# `Diagnostics` |
| ランタイムとメトリクス | GC capability、runtime registry、メトリクス API | Rust runtime hooks, OCaml GC API |
| FFI と安全境界 | `ffi` 効果、`unsafe` ブロック、監査ログ連携 | Rust `std::ffi`, OCaml C stubs |
| プラグインと Capability | DSL プラグイン登録、署名検証、互換性チェック | Rust proc-macro, OCaml PPX |

## 3. 採否・優先度決定の観点
1. **コア哲学との適合度**: 例外非採用／効果タグ／ゼロコスト抽象の原則に従う API を優先し、逸脱する案はガイドラインやラッパを追加して対処する。
2. **横断テーマ貢献度**: Config/Data/Runtime/Diagnostics の共通語彙や監査フローと直接接続する API は優先度を上げる。逆に既存章と重複する機能は Chapter 4 に再配置するだけで十分か検討する。
3. **実装リスクと段階投入**: Async や FFI のような高リスク領域はドラフト仕様から開始し、Core.Parse との整合性やエコシステム依存度を見極めて段階的に正式化する。
4. **互換性と移植性**: OCaml/F#/Rust 等で確立された設計を参照しつつ、Reml の DSL 志向（宣言ビルダー、左→右パイプ）に自然に適合する API かを評価する。
5. **テスト容易性と診断品質**: 単体テスト・プロパティテスト・サンプル DSL で挙動を検証しやすい API を優先し、診断や監査ログの粒度が既存方針と合致するか確認する。

## 4. モジュール候補の分類と優先順位
- **Tier 0（基盤）**: Core.Prelude / Core.Iter / Core.Collections / Core.Text（Unicode）
  - 失敗制御、データ操作、文字列処理はすべての DSL の基礎となるため、Chapter 4 の起点として仕様化する。
- **Tier 1（運用中核）**: Core.Numeric, Core.Time, Core.IO, Core.Path, Core.Diagnostics, Core.Audit
  - Config/Data/Runtime 章で既に利用されている概念の共通 API 化を優先し、CLI・LSP・監査ログとの接続を確立する。
- **Tier 2（横断テーマ連携）**: Core.Config, Core.Data, Core.Runtime, Core.Plugin, Capability Registry
  - 既存章を Chapter 4 配下に再編しつつ、標準ライブラリ観点での再記述・参照整理を進める。
- **Tier 3（将来拡張）**: Core.Async, Core.Ffi, Core.Unsafe
  - 効果タグや安全境界の整合を検証しつつドラフト仕様から開始。外部エコシステムとの接続性・互換性に配慮する。

## 5. フェーズ2 以降への引き継ぎ事項
- README と 0-1/0-2 の章構成を更新し、Chapter 4（標準ライブラリ仕様）の骨子を追加する。
- 既存 Config/Data/Runtime 章の内容を Chapter 4 に再配置する際の差分管理方針を決める（変更履歴・参照更新）。
- Tier 0/Tier 1 モジュールから仕様ドラフトを作成し、効果タグ・診断モデルとの相互参照を明文化する。
- Async/FFI/Unsafe については将来の設計レビュー用に調査メモと互換性ポリシー素案を準備する。

## 6. Config/Data/Runtime 再配置テンプレート

### 6.1 差分作成の手順テンプレート

1. **章ファイルの複製**：`2-7-config.md` / `2-8-data.md` / `2-9-runtime.md` の本文を Chapter 4 用に複製し、`4-8` / `4-8` / `4-9` へ配置。差分を可視化するため元ファイルには「移行済み」ヘッダを追加する。【F:2-7-config.md†L1-L80】【F:2-8-data.md†L1-L50】【F:2-9-runtime.md†L1-L40】
2. **文書先頭の目的文更新**：Chapter 4 側では「標準ライブラリ観点」の解説を追記し、既存ガイドへの逆リンクを整備する。旧 Chapter 2 側は `See Chapter 4.x` のリダイレクト記述に置き換える。
3. **効果タグ・共通語彙の差分整理**：`audit_id` / `change_set` / `capability` など横断語彙を共通節へ集約し、差分ではその節へのリンクを追加する。
4. **サンプルコード検証**：Prelude/Iter/API に沿うよう `use Core;` へ書き換え、Chapter 4 ドキュメントで参照されるコレクション・テキスト API と整合させる。
5. **ドラフトステータスの明示**：各新章に `Status: Draft/Review/Published` メタデータ行を追加し、4.1 の進捗表と同期させる。

### 6.2 リンク更新チェックリスト

- [x] README の Chapter 2 セクションから Config/Data/Runtime への直接リンクを Chapter 4 へ差し替え。
- [x] 0-1 / 0-2 の横断テーマ節で、標準ライブラリ章への参照を追加・更新。【F:0-1-overview.md†L42-L53】【F:0-2-project-purpose.md†L35-L41】
- [ ] Guides (`config-cli`, `runtime-bridges`, `DSL-plugin`) からのリンクを Chapter 4 の該当節へ更新し、旧位置への逆リンクを削除。【F:guides/config-cli.md†L1-L40】【F:guides/runtime-bridges.md†L1-L35】【F:guides/DSL-plugin.md†L1-L50】
- [ ] `2-5-error.md` 内の `Diagnostic` 説明で参照する章番号を Chapter 4 に合わせる。
- [ ] LSP/CLI ドキュメントの索引や目次で `Core.Config` 等の参照先が二重管理になっていないか確認し、必要に応じて脚注で旧番号を明記する。
