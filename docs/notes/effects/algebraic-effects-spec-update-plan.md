# Reml代数的効果 仕様更新計画

> 作成日: 2025年10月 / 策定: Codex
> 対象: Reml v2.0 に向けた仕様書アップデート

## 1. 目的と前提
- 既存仕様は効果をタグ集合として扱い、型システムから切り離している（`1-3-effects-safety.md:11`, `1-3-effects-safety.md:215`）。
- 型推論章では HM（ランク1）と値制限を前提としており、効果型やハンドラの記述が存在しない（`1-2-types-Inference.md:4`, `1-2-types-Inference.md:139`）。
- 構文章の予約語・宣言節に `effect`/`handle`/`do` 等の記述がないため、代数的効果構文を導入する余地が未整備（`1-1-syntax.md:27`, `1-1-syntax.md:131`）。
- ランタイム／標準ライブラリ側はタグベース前提で Capability や Diagnostics を記述しており、継続フレームやハンドラ捕捉の仕様が欠落（`2-6-execution-strategy.md:29`, `3-8-core-runtime-capability.md:10`, `3-9-core-async-ffi-unsafe.md:16`, `3-6-core-diagnostics-audit.md:62`）。
- 既存メモでは段階導入案が提示されており（`docs/notes/effects/algebraic-effects-handlers-spec-proposal.md:73`）、本計画はその documented plan を仕様書へ反映するための編集スケジュールを定める。

## 2. ギャップ整理
| ドキュメント | 現状の重要記述 | 追加/改訂が必要な観点 |
| --- | --- | --- |
| `0-1-project-purpose.md:7` | Reml の価値観を性能/安全/可読性中心で定義。 | Reml らしさを損なわず代数的効果を採用する判断基準を追記。効果ハンドラを導入する際の「段階的習得」指針を明文化。 |
| `1-1-syntax.md:27` `1-1-syntax.md:131` | 予約語と関数宣言のみ。効果構文や `@handles` 属性は未記載。 | `effect` 宣言・`perform/do` 呼び出し・`handle ... with` 構文の導入、属性節でハンドラ契約を追加。 |
| `1-2-types-Inference.md:4` `1-2-types-Inference.md:139` | HM 推論と値制限のみ。効果多相・残余判定が未定義。 | 効果行多相、残余効果の一般化規則、ハンドラ型の型付け規則を追加。 |
| `1-3-effects-safety.md:11` `1-3-effects-safety.md:223` | タグ集合と属性検査。ハンドラでの効果消費や残余計算が未反映。 | 効果宣言の仕組み、残余タグ計算、`@pure`/`@dsl_export` の再定義を統合。 |
| `1-5-formal-grammar-bnf.md` | 現行構文のみ（参照必要）。 | BNF に effect/handle/perform/handler ブロックを追加。 |
| `2-5-error.md:143` `3-6-core-diagnostics-audit.md:109` | 効果契約違反メッセージはタグ集合前提。 | ハンドラ適用後の残余効果差分を表示する診断テンプレートを追加。 |
| `2-6-execution-strategy.md:29` | トランポリンと Packrat の説明のみ。 | ワンショット継続フレーム、ハンドラ実行パス、resume 制限を記述。 |
| `3-1-core-prelude-iteration.md:19` | `@pure` を前提とした API 設計。 | Prelude/Iter に effect 操作の導入方針を補足し、基本 API がどの効果を許容するか明示。 |
| `3-6-core-diagnostics-audit.md:62` | `DiagnosticDomain::Effect` までで停止。 | 効果ハンドラ関連の診断コード、監査メタ情報（捕捉/未捕捉効果）を追加。 |
| `3-8-core-runtime-capability.md:10` | Capability とタグのマッピングが静的。 | 効果宣言の `realm/capability` メタデータとハンドラによる縮退時の検証ステップを追加。 |
| `3-9-core-async-ffi-unsafe.md:16` | `io.*` 効果を直接 API に付与。 | Async/FFI を effect 宣言ベースで説明し直し、ハンドラ経由の置換を許容。 |
| `docs/guides/runtime/runtime-bridges.md` 等 | 実装ガイドはタグ前提。 | 効果ハンドラを用いたモック/非同期連携の手順を追加。 |

## 3. 更新フェーズとタスク

### フェーズA: 価値観と概念整理（2週間）
- **進捗**: 0-1-project-purpose.md と 0-0-overview.md を更新し、段階的導入ポリシーを反映済み。次は B1/B3/B4 着手。
- **進捗**: 0-2, 0-1 の更新完了済み。効果導入ポリシーと段階的習得の記述を追加。
- **A1**: `0-1-project-purpose.md` に代数的効果導入理由と「段階的習得」ポリシーを追記し、Reml らしさとの整合を説明。責任: 設計チーム。
- **A2**: `0-0-overview.md`（参照必要）に効果ハンドラの位置付け（例外/非同期/状態を統一する拡張）を簡潔追加。責任: 編集リード。
- **アウトプット**: ガイドライン更新 diff、レビューガイド。

### フェーズB: コア言語仕様（6〜8週間）
- **進捗**: B1/B2/B3/B4 のドラフトを追加（effect 構文、効果宣言、推論ルール、BNF 反映）。次は診断・ランタイム側の整合を進める。
- **B1**: `1-1-syntax.md` に `effect` 宣言、`perform`/`do` 文、`handle` ブロック構文、`@handles` 属性を追加。構文例と予約語更新。責任: フロントエンド担当。
- **B2**: `1-5-formal-grammar-bnf.md` で新構文を BNF に反映。責任: 文法担当。
- **B3**: `1-2-types-Inference.md` に効果行型、行多相（ランク1 制約）、ハンドラ型の推論規則、残余判定を記述。責任: 型システム担当。
- **B4**: `1-3-effects-safety.md` を再編し、効果宣言（タグと realm）、残余効果計算、`@pure`/`@dsl_export` 判定の新ルール、PoC で利用する `@handles` 属性の意味を確定。責任: 効果仕様担当。
- **レビュー**: B1/B3/B4 同期レビューで整合性確認、`docs/notes/effects/algebraic-effects-handlers-spec-proposal.md` との差分を閉じる。

### フェーズC: 診断・ランタイム・標準ライブラリ（6週間）
- **進捗**: C1/C3/C4 を更新（効果診断・Capability stage 拡張・Async/FFI ハンドラ例）。次は C2/C5 を進める。
- **進捗**: C1/C3 を更新（効果診断・Capability stage 拡張）。次は Async/FFI 章の補強へ。
- **C1**: `2-5-error.md` と `3-6-core-diagnostics-audit.md` にハンドラ境界情報の表示仕様、残余効果差分、監査メタデータを追加。責任: Diagnostics 担当。
- **C2**: `2-6-execution-strategy.md` に継続フレーム管理、ワンショット/マルチショットの扱い、resume 制約を記載。責任: ランタイム担当。
- **C3**: `3-8-core-runtime-capability.md` に効果宣言メタデータと Capability Registry 連携手順、`@reentrant` 等の権限要件を追加。責任: ランタイム担当。
- **C4**: `3-9-core-async-ffi-unsafe.md` を効果ハンドラ前提の説明へ更新（Async=effect宣言、FFI 境界の継続保存ルール）。責任: Async/FFI 担当。
- **C5**: Prelude/Iter (`3-1-core-prelude-iteration.md`) の効果方針追記と `@pure` との両立パターンを提示。責任: ライブラリ担当。

### フェーズD: エコシステム & ガイド（4週間）
- **進捗**: D1 の改訂内容・サンプルを反映済み（runtime-bridges / reml-ffi-handbook 更新、`examples/algebraic-effects/` 追加）。D2/D3 を継続。
- **D1**: `docs/guides/runtime/runtime-bridges.md`, `docs/guides/ffi/reml-ffi-handbook.md` 等のガイドにハンドラ利用方法、モック実装の手順、監査連携を追加。責任: ドキュメント担当。
  - 改訂計画: Async/FFI の stage 昇格フローをガイドへ反映し、3-9 で追加した `collect_logs`/`with_foreign_stub` をベースに実装手順と CLI/Capability 設定例を追記する。必要サンプル: effect ハンドラ差し替えによる async テスト、`ForeignCall` スタブ、stage 昇格チェックの CLI コマンド。
- **D2**: DSL 仕様（`3-7-core-config-data.md`, `3-8`, `notes` 系）で `@dsl_export` と効果宣言の連動を説明（更新済み）。引き続き DSL/LSP 整合をフォロー。責任: DSL 担当。
- **D3**: LSP/IDE 仕様（該当 notes）を更新し、効果ツリー・ハンドラ捕捉状況の可視化要件を追加。責任: ツール担当。

## 4. レビュー・検証
- **技術レビュー**: フェーズB完了後に型/効果/構文の合同レビュー、フェーズC後にランタイム/診断レビューを実施。
- **整合チェック**: 全章の相互参照リンクを更新し、`@pure` や `@dsl_export` の仕様変更が記載ミスを起こしていないか確認。
- **サンプル更新**: 既存の仕様内サンプルコードを効果宣言とハンドラに合わせて更新し、PoC で実行できる最小例を添付。
- **バージョン管理**: 各フェーズ終了時にタグ付きドラフトを発行し、コミュニティレビューを募集。

## 5. リスクと対策
| リスク | 対象フェーズ | 緩和策 |
| --- | --- | --- |
| 導入文書が冗長化し、「Remlらしさ」を損なう | A/B | 構文・型章では導入ストーリーを最小限に留め、詳細は補遺またはガイドへ分離。編集レビューでトーンを統一。 |
| `@pure` 判定再定義の齟齬 | B/C | 効果章と型章で同じ用語を使用し、レビュー用チェックリストを用意。PoC テストの結果を仕様脚注に記録。 |
| Capability/診断の更新漏れ | C/D | 変更箇所の一覧を維持し、各章の更新後に cross-check meeting を実施。 |
| 時間超過 | 全体 | フェーズごとに進捗チェックを設定し、リソース不足の場合はガイド更新を後ろ倒しする。 |

## 6. 次アクション（2週間以内）
1. 2025-11-05 14:00 JST の合同レビューを正式案内し、参加者へ `docs/notes/effects/algebraic-effects-review-checklist.md` と差分資料を送付する。
2. フェーズD-2/D-3 の作業割り当て（DSL/IDE 仕様の整合項目）を整理し、担当者とスケジュールを確定する。
3. `examples/algebraic-effects/` を CI に組み込むための簡易スクリプトをドラフト化し、次回レビューで共有する。

---
本計画に従って仕様書を更新することで、Reml の既存価値（性能・安全・DSL ファースト）を維持しつつ、代数的効果ハンドラを正式仕様へ統合する基盤を整備する。
