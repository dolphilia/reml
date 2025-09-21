# Reml 仕様改良導入計画（2024Q4-2025Q1）

## 1. 背景と目的
- `research-Reml-latest.md` で整理した最新研究動向を受け、Reml の言語仕様を段階的に強化する。
- 既存仕様との整合を維持しつつ、IDE/LSP 連携や国際化対応における即効性の高い改善を優先する。
- 本計画は Phase1（0-3 ヶ月）に着手するタスク群を対象とし、合意形成と着手判断の拠り所を提供する。

## 2. 改良テーマと導入判断
### 2.1 Unicode 処理刷新
- **現状課題**: 仕様は UAX #29/31 を前提としているが、Unicode 15.1/ICU の更新サイクルと同期した基準値・テスト手順が未定義 (`1-4-test-unicode-model.md:71`, `1-4-test-unicode-model.md:192`, `1-4-test-unicode-model.md:215`)。
- **導入メリット**: 表示幅推定や confusable 検査の精度向上、仕様の将来拡張に備えたデータ更新フローの明文化。
- **懸念点**: ICU4X/`ugrapheme` の導入による依存増とバイナリサイズ、OSS ライセンス確認の必要性。
- **対策案**: PoC で CPU/メモリインパクトを測定し、最終採択ライブラリを評価指標と共に仕様へ明記。

### 2.2 “エラー不可能”パーサ結果への移行
- **現状課題**: `run` 系 API は `Result<(T, Span), ParseError>` を返し、`recover` 利用時のみ AST に `ErrorNode` を挿入する (`2-1-parser-type.md:155`, `2-5-error.md:195`)。
- **導入メリット**: IDE での部分 AST 活用、FixIt 提案の一貫化、`toDiagnostics` と AST 生成の二重実装解消。
- **懸念点**: ランナー API の破壊的変更による移行コスト、互換レイヤ設計が必須。
- **対策案**: `ParseResult { value: T, diagnostics: List<Diagnostic>, recovered: Bool }` の草案を提示し、旧 API を `cfg.legacy_result=true` で併存させる移行期間を設ける。

### 2.3 期待集合の人間語化と LSP 連携
- **現状課題**: `Expectation` はトークン/ルールの列挙に留まり、文脈情報や提示順序の規約が不足 (`2-5-error.md:33`, `2-5-error.md:71`, `2-5-error.md:224`)。
- **導入メリット**: IDE/LSP 診断の可読性向上、`FixIt` 提案との紐付け強化、国際化されたメッセージ生成の基盤整備。
- **懸念点**: JSON 出力スキーマ更新に伴うツール互換性チェック、翻訳用メッセージ ID の管理コスト。
- **対策案**: `Expectation` 拡張（例: `Alternative`, `ContextNote`, `MessageKey`）をプロトタイプで検証し、`PrettyOptions` と LSP schema を同時更新。

## 3. 実施ロードマップ（Phase1 重点）
| 期間 | テーマ | 主担当 | 成果物 | 完了判定 |
| ---- | ------ | ------ | ------ | -------- |
| W1-W2 | Unicode 処理刷新 | Core.Text チーム | PoC レポート（CPU/メモリ測定、ライセンス確認） | ICU4X/`ugrapheme` の採用可否レビュー完了 |
| W3-W5 | Unicode 仕様更新 | Core.Text/Docs | `1-4-test-unicode-model.md` 改訂、テストデータ付録 | 仕様 PR 承認 + テストケース追加 |
| W2-W4 | エラー不可能 PoC | Core.Parse チーム | `ParseResult` API 草案、互換ポリシー案 | プロトタイプで `(AST, Diagnostics)` を返すランナー実装デモ |
| W4-W6 | エラー仕様文書化 | Core.Parse/Docs | `2-1-parser-type.md`/`2-5-error.md` 改訂草案 | 設計レビューで互換戦略合意 |
| W3-W5 | 期待集合拡張設計 | IDE/LSP & Core.Parse | `Expectation` 拡張仕様、LSP JSON スキーマ案 | LSP プロトコルでサンプル診断が往復確認済み |
| W6 | フェーズレビュー | All | 進捗共有資料、次フェーズ課題表 | Phase2 テーマ（型推論/インクリメンタル）への移行判断 |

## 4. タスク分解と依存関係
- Unicode PoC 完了が `1-4` 改訂と confusable テスト整備の前提。
- `ParseResult` 設計は `Expectation` 拡張仕様と同時レビュー（診断構造を共有するため）。
- LSP スキーマ更新は CLI ツール (`reml-run`, `reml-config`) の JSON 出力変更と同期させる。

## 5. 評価指標
- Unicode: 代表 10 ケース（絵文字連結・Bidi 混在・全角幅）で表示幅誤差ゼロ、confusable 警告の誤検知率 < 2%。
- エラー不可能: recover 有無に関わらず AST 生成率 100%、既存テストでの Diagnostics 差分をレビューで許容範囲に収束。
- 期待集合: LSP 経由の診断メッセージで期待候補提示が 3 ステップ以内に理解できるかを UX 評価（既存課題との差分を定性評価）。

## 6. リスクと緩和策
- **依存ライブラリ更新が遅延**: ICU4X のリリースが遅れた場合に備え、`rust-unic` + 手動テーブル更新のバックアップ案を保持。
- **API 互換性問題**: 旧 `Result` に依存するツール向けに、一定期間 `ParseResult::legacy()` を提供し警告付きで移行を促す。
- **翻訳作業の負荷**: `MessageKey` を導入し、初期段階では日本語/英語のみ提供、他言語は Phase2 以降に拡張。

## 7. コミュニケーションとレビュー
- 毎週火曜の Core Sync で進捗共有、各テーマのレビュー締切を明示。
- 設計ドキュメントは `notes/` 以下にドラフトを配置し、PR テンプレートに研究ノートの参照箇所を記載。
- IDE ベンダー（社内 LSP/補助ツール担当）との共有会を Phase1 終盤に設定。

## 8. 次フェーズへの布石
- Phase2 では双方向型付け・Simple-sub の検証、Salsa 互換のインクリメンタル解析を重点検討する。
- Phase1 で収集したメトリクスと UX フィードバックを Phase2 企画書の基礎データとする。

---
この計画書は 2024 年 Q4 の仕様更新キックオフ資料として用い、決議後は `spec-update-plan.md` に要約を反映する。
