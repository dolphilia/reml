# Phase4: Core.Parse Input/Zero-copy 実装計画

## 背景と目的
`Core.Parse` は仕様上すでに `Input` を「参照共有の不変ビュー（ゼロコピー）」として定義している（`docs/spec/2-1-parser-type.md`）。
一方で Phase4（spec_core 回帰）では、診断・回復・Packrat 等の改善が進むほど `Input` の派生（`rest`/`mark/rewind`）がホットパス化し、**部分文字列生成**や **Unicode 位置の都度走査**が混入すると性能と診断表示の両方が崩れうる。

本計画は、WS5（Input/Zero-copy）の決定事項を Phase4 実装へ接続し、性能指針（10MB 線形、メモリ 2x 以内）を守れる状態へ段階的に導くための作業導線を提供する。

- 出典（WS5）: `docs/plans/core-parse-improvement/1-4-input-zero-copy-plan.md`
- 設計指針: `docs/spec/0-1-project-purpose.md`

## スコープ
- 対象: Rust 実装の `Core.Parse` 入力モデル（`Input`/`Span`/`mark/rewind`/メモ化キー）と、それに直結する診断ハイライト（列=グラフェム）
- 非対象:
  - 新しい入力モデルの発明（仕様が求める前提へ “戻す” のが先）
  - API の全面置換（Phase4 の回帰を壊さず段階導入する）
  - 他実装の追従・差分解消（Phase4 では Rust 実装に注力する）

## 仕様根拠
- `docs/spec/2-1-parser-type.md`（入力モデル `Input`、`mark/rewind`、`MemoKey`）
- `docs/spec/3-3-core-text-unicode.md`（列=グラフェム、`Span` ハイライト整合、`g_index/cp_index` 再利用）

## 成果物（Phase4 観点）
- チェックリスト（実装監査の基準）:
  - `docs/plans/bootstrap-roadmap/checklists/core-parse-input-invariants.md`
- 回帰シナリオ（Phase4）:
  - `CP-WS5-001`（大入力オーダー異常）: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`
  - `CP-WS5-002`（Unicode 位置）: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`
- 実行ログ（運用）:
  - `reports/spec-audit/ch5/logs/` に、CP-WS5-001/002 の再実行ログを残す（手順は Step3 で規定）

## 実装対象（Rust）
| 領域 | 主ファイル | 目的 |
| --- | --- | --- |
| `Input`/`Span`/位置更新 | `compiler/runtime/src/parse/combinator.rs` | ゼロコピー前提・境界安全・位置算出のコストを制御する |
| Span ハイライト（列=グラフェム） | `compiler/runtime/src/text/span_highlight.rs` | Unicode 行での下線/列計算を安定させる |
| Phase4 回帰資産 | `examples/spec_core/chapter2/parser_core/*` / `expected/spec_core/chapter2/parser_core/*` | CP-WS5-001/002 を回帰として固定する |
| マトリクス登録 | `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` | 期待キー/期待出力/運用ノートを一元化する |

---

## Step 0（前提・完了済み）: Input 不変条件チェックリストを確定する
仕様が要求する不変条件を **監査可能なチェック項目**へ落とす（WS5 Step0）。

- 成果物:
  - `docs/plans/bootstrap-roadmap/checklists/core-parse-input-invariants.md`
- 完了判定:
  - `rest`/`mark/rewind`/Unicode 位置/診断整合/Packrat とメモリ上限について、最低限の “逸脱検知” ができる

## Step 1（必須）: 実装是正の作業項目を確定し、最小修正から入れる
WS5 Step1 の監査結果を起点に、Phase4 の回帰を壊さずに “逸脱” を潰す。

### 1-1. 入力二重確保（`Input::new` の全体コピー）を除去する
**狙い**: 10MB 級入力でも「入力サイズの 2 倍以内」を破りにくくする（`docs/spec/0-1-project-purpose.md`）。

- 現状論点: `Input::new(&str)` が `Arc<str>` へ変換するため、入力全体を複製しやすい（監査メモ参照）。
- 作業対象（例）:
  - `compiler/runtime/src/parse/combinator.rs`（`Input` / `ParseState::new` / `run`）
- 作業案（優先順）:
  1. `Input` へ “既に共有されているバッファ” を渡すコンストラクタを追加する（例: `Input::from_arc_str(Arc<str>)`）
  2. `run` の入口で入力バッファを 1 回だけ確保し、`Input` は参照共有として受け取る（`&str`→`Arc<str>` の再確保を避ける）
  3. 仕様側の最終形（`Bytes` + `byte_off` + `cp_index/g_index`）へ寄せるのは次段（Phase4 では “二重確保回避” を先に固定）
- 進捗（2025-12-18）:
  - `Input::from_arc_str` / `ParseState::new_shared` / `run_shared` を追加し、共有済みバッファを全体コピーなしで受け取る経路を用意した（Rust runtime）。
  - 既定の `run(&str)` は従来どおり `Input::new` を経由するため、フロントエンドからの呼び出し置換（`run_shared` 採用）が後続課題。
- 完了判定:
  - 大入力（CP-WS5-001）を想定した実行で、入力バッファの二重確保が必須になっていない（実装レビューで根拠を示せる）

### 1-2. UTF-8 境界前提の “暗黙” を API で封じる
**狙い**: `byte_offset` が UTF-8 境界でない場合に panic しうる経路を、将来の変更で踏まないようにする。

- 現状論点: `remaining()` が `&source[byte_offset..]` を返すため、誤用で panic しうる（監査メモ参照）。
- 作業対象（例）:
  - `compiler/runtime/src/parse/combinator.rs`（`Input::remaining` / `Input::advance`）
- 作業案（例）:
  - `advance(bytes)` を「境界前提 API」として残す場合でも、`debug_assert!(source.is_char_boundary(...))` 等で境界を検証する（少なくとも debug ビルド）。
  - `advance` を “bytes 指定” と “token/str 指定” に分け、境界が保証できる経路をデフォルトにする。
  - `remaining()` は `get(byte_offset..)` を用いた安全な参照を返す（失敗時は空スライス扱いにせず、呼び出し側へ失敗を返すのが望ましい）。
- 進捗（2025-12-18）:
  - `remaining_checked` で `get` ベースの安全参照を追加し、`remaining` に境界違反の `debug_assert` を導入。
  - `advance` に UTF-8 境界ガードを追加し、誤用時にデバッグで検知できるようにした。
- 完了判定:
  - `Input` の public API が「境界を守るべき責務」をコードで表現できている（レビューで説明できる）

### 1-3. 位置更新（列=グラフェム）の都度走査を減らす
**狙い**: 位置更新とハイライトが、大入力×多数診断のときに O(n*m) 化しないよう “穴” を塞ぐ。

- 現状論点:
  - `advance` が `char_indices()` + `iter_graphemes().count()` を都度走査する
  - `span_highlight` が行頭探索を先頭から走査する
- 作業対象（例）:
  - `compiler/runtime/src/parse/combinator.rs`（位置更新）
  - `compiler/runtime/src/text/span_highlight.rs`（ハイライト生成）
- Phase4 での最小方針:
  - 「キャッシュを入れる」より先に、**二重走査を避ける**（字句境界探索→advance で同じ範囲を再走査しない）
  - 位置算出の責務を `Core.Text` 側へ寄せる（仕様の方針に合わせる）
- 進捗（2025-12-18）:
  - `advance`/`span_highlight` に ASCII fast path を追加し、グラフェム走査を必要最小限に分岐。
  - 行頭テーブルや `g_index/cp_index` 共有によるキャッシュは未着手（次段で検討）。
- 完了判定:
  - CP-WS5-001 の 10MB 実行（回復 OFF/ON）で、`packrat_stats` 等の増え方が極端に悪化しない（Step2 の観測で確認できる）

### Step1 の根拠（監査メモ）
- `docs/notes/parser/core-parse-api-evolution.md` の `2025-12-18: WS5 Step1 Input/Zero-copy 実装監査メモ（Rust runtime）`

---

## 調査メモ（2025-12-19）: 大入力での stack overflow
- 症状: `tooling/examples/gen_ws5_large_input.py --sizes 5mb,10mb` で `reml_frontend --output json --emit-diagnostics` が stack overflow（exit=-6）。`RUST_MIN_STACK=64/128MB` 指定でも解消せず。
- 再現範囲: 1KB/100KB/1MB は通過。5MB 以上でクラッシュ。
- lldb で単体バイナリを実行すると `core::str::len` で EXC_BAD_ACCESS（stack guard 侵害）で停止。`logos` の `lex`→`Skip`→`lex` が再帰的に呼ばれ、8万フレーム超まで積み上がることを確認（再帰爆発が原因）。
- 状態: CP-WS5-001 の 10MB ケースは未固定。1MB までのログは `reports/spec-audit/ch5/logs/` に取得済み。
- 進捗（2025-12-19）:
  - `lex_source_with_options` をオフセットループ化し、空白/コメントの手動スキップを追加して `logos` の Skip 再帰を排除（5MB 入力が通ることを確認）。
  - `gen_ws5_large_input.py` に `--streaming-fallback`（`--stream-demand-*` 付き再実行）を追加し、クラッシュ時のフォールバック実行が可能になった。
  - 10MB 入力も通常実行で通過し、`reports/spec-audit/ch5/logs/spec_core-CP-WS5-001-10mb-20251218T211703Z.diagnostic.json` を取得。
  - 代表ケース（`core-parse-unicode-grapheme-column.reml` / `core-parse-large-input-order.reml`）は診断の主要フィールド一致を確認（差分は run_id 等のメタ情報のみ）。

### 修正に向けた具体ステップ案
1. バックトレース確保  
   - `lldb compiler/frontend/target/debug/reml_frontend -- --output json --emit-diagnostics <5mb>` を用い、`process handle SIGUSR1 -n false -p false -s false` 設定後に `thread backtrace all` を取得。再帰元/ループ箇所を特定。
2. ストリーミング/チャンク実験  
   - `stream.enabled=true` または `chunk_size` を RunConfig/CLI で指定して 5MB/10MB を再実行し、スタック深度を抑制できるか確認。成功パターンがあれば Step2 の運用手順にフォールバック案として追記。
3. 実装修正（候補）  
   - Backtrace で特定した関数の再帰を iterative 化、または診断生成の深さ/トークン列走査を早期切り上げ。巨大入力での線形近似を保つように修正。
4. スクリプト改善  
   - `gen_ws5_large_input.py` にフォールバック実行オプション（例: `--stream --chunk-size …`）を追加し、クラッシュ時もログ収集を試行する。
5. 回帰メモ更新  
   - 5MB/10MB の結果（成功/失敗を含む）を `expected/spec_core/chapter2/parser_core/core-parse-large-input-order.expected.md` と `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` に記録し、未完了である旨を明示。

### 残タスク（2025-12-19 時点）
- `core-parse-unicode-grapheme-column.diagnostic.json` はメタ差分のみのため **保持**（更新不要）と判断。必要になった場合のみ run_id 等の差分をまとめて更新する。

---

## Step 2（必須）: 観測と回帰の “固定方法” を決める（既存の出力を優先）
性能回帰は CI 差が大きいため、初期は「絶対時間」より **増え方（オーダー異常）**を固定する。

### Step2 で優先する観測（Phase4 の既存 JSON に含まれる項目）
追加の計測 API を先に作らず、`reml_frontend --output json` がすでに出力している統計を利用する。

- `summary.stats.parse_result.packrat_stats`（hits/queries/entries/approx_bytes/evictions 等）
- `summary.stats.parse_result.packrat_snapshot`（entries/approx_bytes）
- `summary.stats.parse_result.farthest_error_offset`
- `diagnostics[].location`（大入力でも line/column が破綻していないか）

回帰（CP-WS5-001）では、入力サイズ（1KB/100KB/10MB）の増加に対して、上記の統計が不自然に跳ね上がらないこと（オーダー異常がないこと）を “まず” 固定する。

### Step2 の実施手順（運用・ログの残し方）
1. 生成→実行→ログ保存は `tooling/examples/gen_ws5_large_input.py` に集約する（手作業で 10MB ファイルを作らない）
   - 実行例: `python3 tooling/examples/gen_ws5_large_input.py --sizes 1kb,100kb,10mb`
   - フォールバック実行（任意）: `python3 tooling/examples/gen_ws5_large_input.py --sizes 1kb,100kb,10mb --streaming-fallback --stream-chunk-size 65536`
   - 生成入力の出力先: `reports/spec-audit/ch5/generated/ws5/CP-WS5-001/`
   - 実行ログの出力先: `reports/spec-audit/ch5/logs/`（例: `spec_core-CP-WS5-001-10mb-YYYYMMDDTHHMMSSZ.diagnostic.json`）
2. ログ JSON から `packrat_stats` と `farthest_error_offset` を確認し、必要なら `expected/spec_core/chapter2/parser_core/core-parse-large-input-order.expected.md` に追記する（`--update-notes` を使って自動追記も可）
3. Step1 の変更前後で “増え方” が悪化していないかをレビューし、`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CP-WS5-001.resolution_notes` にログパスと RunConfig 前提（packrat/recover/stream 等）を残す

補足:
- 追加の観測フック（`RunConfig.profile` 等）を使う場合でも、まずは上記 “既存 JSON” を基準にする（比較が容易で、Phase4 の運用に載りやすい）。

---

## Step 3（必須）: Phase4 シナリオへ接続し、期待出力を固定する
Phase4 の回帰資産として、次の 2 本立てで固定する（登録済み）。

### CP-WS5-001（大入力オーダー異常）
- 入力（生成ベース）: `examples/spec_core/chapter2/parser_core/core-parse-large-input-order.reml`
- 期待（アンカー）: `expected/spec_core/chapter2/parser_core/core-parse-large-input-order.diagnostic.json`
- 10MB 生成・観測メモ: `expected/spec_core/chapter2/parser_core/core-parse-large-input-order.expected.md`
- シナリオ行: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CP-WS5-001`
- 完了判定（Phase4）:
  - 1KB/100KB/10MB の観測ログを `reports/spec-audit/ch5/logs/` に保存できる
  - `resolution_notes` に「どの RunConfig（packrat/recover/profile）」で測ったかが残っている
  - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CP-WS5-001.resolution` を `ok` に更新できる

### CP-WS5-002（Unicode 位置）
- 入力: `examples/spec_core/chapter2/parser_core/core-parse-unicode-grapheme-column.reml`
- 期待: `expected/spec_core/chapter2/parser_core/core-parse-unicode-grapheme-column.diagnostic.json`
- シナリオ行: `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CP-WS5-002`
- 完了判定（Phase4）:
  - `diagnostics[].location.column` と highlight が Unicode（ZWJ 絵文字等）で崩れないことを、expected で固定できている
  - 実装変更で揺れた場合に、どの不変条件（チェックリスト）に抵触したかへ差し戻せる
  - `docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` の `CP-WS5-002.resolution` を `ok` に更新できる

---

## 既知のリスクと緩和
- **性能計測が環境差で揺れる**: 時間の絶対値を合否に使わず、`packrat_stats` 等の “増え方” を優先する
- **Unicode 位置の固定が brittle**: 列=グラフェムの定義を `Core.Text` に寄せ、実装の独自ロジックを増やさない（`docs/spec/3-3-core-text-unicode.md`）
- **大入力のゴールデン管理が重い**: 10MB 本体を `expected/` にコミットせず、生成手順と観測ログ（`reports/`）で追跡する
