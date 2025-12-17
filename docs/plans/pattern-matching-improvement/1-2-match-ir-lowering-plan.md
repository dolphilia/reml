# 1.2 Match/Pattern IR ローアリング計画

## 目的と背景
- 現状は TypedExpr/TypedPattern で Active/Or/Slice/Range/Regex/Binding を正規化しているが、MIR/LLVM などバックエンドへ伝搬する IR が存在せず、Partial Active の `None` フォールスルーや Range/Slice/Or の分岐生成が未実装。
- Phase 4 の網羅性/診断マトリクス（`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv`）を完了させるには、コード生成側で分岐を組み立て、expected を再取得する必要がある。
- 本計画では IR 仕様の決定 → MIR 生成 → バックエンド変換 → 回帰資産更新までの手順をまとめ、Rust/OCaml 実装間の差分を吸収しやすくする。

## スコープ
- **含む**: MIR への Match/Pattern ノード追加、Active partial の miss パス設計、Range/Slice/Or の分岐マッピング、Phase4 マトリクス/expected の再取得手順。
- **含まない**: LSP 連携 UI、最適化パス（ジャンプ圧縮・ヒューリスティクス）、OCaml バックエンド実装の詳細。

## 現状のギャップ
- フロントエンド出力: `match_lowerings` をデバッグ用に生成するのみで、バックエンド入力となる IR が未定義。
- バックエンド: MIR JSON ローダー（`compiler/rust/backend/llvm/src/integration.rs`）は関数シグネチャのみを扱い、Match/Pattern ノードが存在しない。
- 期待資産: Partial Active miss・Range/Slice/Or 分岐を含む expected/diagnostic の再取得が未実施（run_id の記録もなし）。

## 具体的な作業ステップ
1. **IR 仕様策定 (MIR 拡張案を決定)**  
   - 非終端: `MirExpr::Match { target, arms }`, `MirMatchArm { pattern: MirPattern, guard, alias, body }`。  
   - パターン: `Wildcard | Var | Literal | Constructor | Tuple | Record | Binding(@/as) | Or(Vec) | Slice(Vec<Elem|Rest>) | Range { start, end, inclusive } | Regex { pattern } | Active { name, kind: Partial/Total, arg }`。  
   - Partial Active: `Active { kind=Partial }` は `miss_target` を保持し、`None` なら次アームへジャンプする構造を MIR で表現する。
   - Guard/Alias: 順不同受理だが MIR では `guard -> alias -> body` の評価順を固定する。

2. **フロントエンド MIR 生成の実装**  
   - `TypedExprKind::Match` から上記 MIR を構築するパスを追加。`match_lowerings` 相当の情報を MIR ノードに格納する。  
   - Partial Active の miss パスは `on_miss: Label` を付与し、Range/Slice/Or はノード種別で判別可能にする。  
   - JSON エクスポート: デバッグ用に MIR を `reml_frontend` から出力できるサブコマンド（または `--debug-mir`）を追加。

3. **バックエンド変換 (MIR → LLVM) の追加**  
   - `integration.rs` で MIR JSON に `Match`/`Pattern` を読み込み、`CodegenContext` でジャンプ命令を組み立てる。  
   - 部分 Active: `Some` → 現在アーム本体、`None` → 次アームへ直接ジャンプ。  
   - Range: 比較演算とブール合成で分岐生成（`..=` を含む）。  
   - Slice: 先頭/末尾固定要素と Rest の長さチェックを分岐化。  
   - Or: 各バリアントを順に評価し、成功で抜けるフォールスルー構造を生成。

4. **回帰資産の再取得とマトリクス更新**  
   - 対象: `examples/spec_core/chapter1/match_expr/*.reml`（Range/Slice/Or/Active miss 含む）および `active_patterns/*.reml`。  
   - コマンド記録: Phase4 マトリクスの該当行（CH1-ACT-00x / CH1-MATCH-01x）に run_id・再取得コマンドを追記し、diagnostic.message がレジストリ文面に一致することを確認。  
   - IR ログ: `reports/spec-audit/ch4/logs/` に MIR/LLVM 簡易差分を保存し、ギャップがあればコメントで理由を残す。

5. **クロスチェックとドキュメント同期**  
   - 仕様: 必要に応じて `docs/spec/1-5-formal-grammar-bnf.md` の Match/Pattern セクションに「Partial Active miss は `None` で次アームへ」等の注記を追記。  
   - ガイド: `docs/guides/core-parse-streaming.md` 付録で Match 分岐の実行順とストリーミング評価の関係を整理。  
   - OCaml 実装: MIR 仕様が確定したら差分メモを `docs/plans/rust-migration/` へ残し、移植方針を共有。

## M1: MIR 拡張仕様（決定）

### 基本ノードとメタデータ
- `MirExpr::Match { target, arms, span, ty }` を追加し、`target` は 1 度だけ評価して全アームに共有する。`ty` は `match` 式の戻り型を保持し、分岐先の型不一致を防ぐ。  
- `MirMatchArm { pattern, guard, alias, body, span }` で表現し、ガード失敗時は次アームへフォールスルーする前提とする。`guard` は `Option<MirExprId>`、`alias` は `Option<BindingId>`、`body` はブロック/式 ID を保持。  
- すべてのパターンに `span` と型ラベル（型検査済みのラベル ID）を付与し、診断とバックエンド最適化で再利用する。`dict_ref_ids` / 効果集合は既存 MIR ノードと同じメタデータスキーマを継承する。

### パターン種別と表現
- `Wildcard | Var { name } | Literal { value } | Tuple { elements } | Record { fields, has_rest } | Constructor { path, args }` を既存 TypedPattern と同等に持ち込み、構造はフロントエンド正規化結果をそのまま使用する。  
- Binding は `Binding { binder, inner }` に統一し、`pat as x` / `x @ pat` の両記法をここで集約する。`binder` は束縛名、`inner` は元パターン。  
- Or パターンは `Or { variants: Vec<MirPattern> }` とし、ネストはフラット化済みを前提にする。各バリアントの型は同一でなければならない（型検査済み）。  
- Slice パターンは `Slice { head: Vec<MirPattern>, rest: Option<BindingId>, tail: Vec<MirPattern> }` で保持し、`rest` が `None` なら固定長、`Some` なら可変長。長さ検証は MIR/LLVM で分岐化する。  
- Range パターンは `Range { start, end, inclusive }`。`start/end` はリテラルまたは識別子参照を許容し、両者の型一致が前提。`inclusive=true` は `..=`、`false` は `..`。  
- Regex パターンは `Regex { pattern, flags }` とし、ターゲット型が文字列/バイト列以外の場合は前段で診断済み。バックエンドではパターン全体一致（アンカー付き）を既定とする。  
- Active パターンは `Active { name, kind: Partial|Total, args, input_binding, miss_target }`。`args` はマッチ対象以外の引数列、`input_binding` は呼び出し結果をアーム内で再利用する場合の束縛、`miss_target` は Partial のみ `Some(Label)` とし、`None`/`Some` を MIR 上でブランチに変換する。

### 評価順序と制約
- アーム評価順は **パターン照合 → ガード → エイリアス → 本体** に固定する。ガードは `when/if` いずれで記述されても MIR では `guard` に正規化し、`alias` はガード通過後にのみ束縛が有効になる。  
- Partial Active は `None` → `miss_target` へジャンプ、`Some(payload)` → 残りのパターン照合/ガードへ進む。Total Active は常に成功とみなし、payload を `input_binding` へ束縛する。  
- Or/Range/Slice/Regex/Binding で生成される補助分岐は MIR レベルで表現し、LLVM 変換ではジャンプ命令として具体化する。  
- すべてのパターン比較は副作用なしを前提とし、効果が発生するのは Active Pattern 本体のみ。`@pure` 文脈では Typeck 段階の制約を尊重し、MIR には効果マーカーを追加しない。

### JSON/バックエンド向け契約
- `reml_frontend --debug-mir` で出力する JSON では、`expr.kind = "Match"` の下に `target`（式 ID）、`arms`（各アームに `pattern.kind` 等）を格納する形を基準とする。  
- Active Pattern の miss 分岐は JSON でも `miss_target` を明示し、バックエンドが Option 判定なしにジャンプを構築できるようにする。  
- 今回の決定は `docs/plans/bootstrap-roadmap/4-1-spec-core-regression-plan.md` の Phase4 マトリクスに沿った診断（Active miss / Range/Slice/Or 分岐）を LLVM まで伝搬させることを目的とし、以降のステップではこのスキーマを変更しない前提で実装を進める。

## 優先順位とマイルストーン
- **M1: IR 仕様決定** — 上記 MIR ノード案をレビューし、受理。  
- **M2: フロントエンド MIR 生成** — `TypedExprKind::Match` から MIR 生成が通り、JSON エクスポートができる状態。  
- **M3: バックエンド分岐生成** — Partial Active miss と Range/Slice/Or/Regex/Binding を含むサンプルで LLVM 風出力が生成される。  
- **M4: 回帰資産更新** — expected/diagnostic 再取得・マトリクス run_id 記録完了。  
- **M5: クロス実装チェック** — OCaml/回帰レポートとの差分メモ完了。

### 進捗ステータス（2026-03 時点）
- [x] M1: IR 仕様決定 — 本計画記載のノード/評価順を採用。  
- [x] M2: フロントエンド MIR 生成 — `compiler/rust/frontend/src/semantics/mir.rs` を新設し、`TypedExprKind::Match` から `MirExpr::Match` を構築。`--emit-mir`/`--debug-mir` オプションで JSON 出力できるように `compiler/rust/frontend/src/bin/reml_frontend.rs` を更新。`TypecheckReport` に MIR を保持しデバッグ出力へ組込み済み。**追加完了**: `FieldAccess`/`TupleAccess`/`Index` を TypedExpr/MIR に連携、`Some/None/Ok/Err/format`/`to_string`/`len`/`is_empty`/`starts_with`/`push`/`pop` などの簡易型推論を実装し、CH1-MATCH/ACT サンプルの `--emit-mir` で `Unknown` が 0 件になる状態を確認。  
- [ ] M3: バックエンド分岐生成 — **進行中（更新: 2025-12）**。`CodegenContext` が Range/Slice/Or/Partial Active/Guard/Alias/Body の順で分岐を展開し、`LlvmBlock`（`LlvmInstr`/`LlvmTerminator`）を `LlvmFunction` として組み立て、`llvm_ir` をスナップショットに格納するところまで到達。追加で、`LlvmFunction/LlvmBlock` の組み立てを `LlvmIrBuilder` に集約し、`emit_function` から一貫して `ModuleIr` に格納される経路を固定した。  
  - **確認済み**: `tmp/mir-bnf-match-range-inclusive-ok.json` / `tmp/mir-bnf-match-slice-head-tail-ok.json` / `tmp/mir-bnf-activepattern-partial-ok.json` などの MIR JSON を入力に `llvm_ir` をダンプし、`icmp`/`and`/`br`/`phi` がブロック列として出力されることを確認。  
  - **修正**: `Or` / フォールバック分岐の `br` 条件が「未定義 SSA（new_tmp しただけ）」になっていた箇所を、条件 SSA の生成へ置換。あわせて Constructor/Regex/Binding/Wildcard/Var は `@reml_match_check` ではなく専用の条件生成（`icmp`/専用 `call`）へ移行し、CH1-MATCH/ACT の範囲で `@reml_match_check` 依存を縮小。  
  - **追加**: Guard を `MirExprKind::Binary/Identifier/Literal` の範囲で SSA 化し、`arm{n}.guard#...` の `br` 条件に接続。Body も `Literal/Identifier` を SSA（`@reml_value` 呼び出し）として生成し、`match.end` の `phi` incomings に供給。  
  - **ログ**: 一括ダンプ結果を `reports/spec-audit/ch4/logs/match-ir-20251217T023255Z.md` に保存（Range/Slice/Or/Partial Active/Guard/Body の接続を目視確認）。  
- [ ] M4: 回帰資産更新 — 未着手。CH1-MATCH/ACT の expected 再取得と run_id 記録が必要。  
- [ ] M5: クロス実装チェック — 未着手。Rust/OCaml 差分メモ作成が必要。

### 次に着手する作業（優先順）
1. **Constructor の内側パターンに対応**: `Some(x)` 等の payload を取り出して `Binding`/`Var`/`Literal` へ伝搬する（現状は `Some/None` を null 判定で分岐するのみ）。  
2. **Guard/Body の式評価を拡張**: `Call` / `IfElse` / `FieldAccess` などを最小限で扱い、`#id` フォールバックを減らす（CH1-MATCH/ACT の残りサンプルで `match_result <- #...` が出ない状態を目標）。  
3. **M4 へ接続**: Phase4 マトリクス該当行（CH1-ACT-00x / CH1-MATCH-01x）で expected/diagnostic を再取得し、run_id と再取得コマンドを記録する（`docs/plans/bootstrap-roadmap/assets/phase4-scenario-matrix.csv` を更新）。  
4. **@reml_* の整理**: `@reml_value` / `@reml_regex_match` など「LLVM 風 IR での暫定コール」を `runtime_link`/FFI 宣言の方針に合わせて整理し、将来の実 LLVM IR 生成へ移行しやすい境界を固定する。

## 退出条件
- Match/Pattern を含む MIR が生成され、バックエンドでジャンプ分岐が構築される。Partial Active の miss パスがランタイム挙動として確認できる。  
- Phase4 マトリクスの該当行に最新 run_id・再取得コマンドが記録され、expected が更新済み。  
- IR 仕様とドキュメント（必要箇所）が同期され、将来の OCaml 実装に転記できる状態。 
