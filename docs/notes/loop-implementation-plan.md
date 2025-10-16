# ループ構文実装計画

**作成日**: 2025-10-13
**Phase**: Phase 2 Week 20-21
**関連タスク**: [2-1-typeclass-strategy.md](../plans/bootstrap-roadmap/2-1-typeclass-strategy.md) セクション4

## 概要

型クラスベンチマーク実行のために while/for ループの実装を試みたところ、現在のコンパイラアーキテクチャにおける構造的課題が明らかになりました。本文書では、発見された課題と、Phase 3 以降での完全実装に向けた戦略を記録します。

## 現状の実装状況

### ✅ 完了した実装（Phase 2）

| コンポーネント | ファイル | 実装内容 | 完了度 |
|--------------|---------|---------|--------|
| AST定義 | `src/ast.ml:106-108` | `While`, `For`, `Loop` 式の定義 | 100% |
| パーサー | `src/parser.mly:718-737` | while/for/loop のパース規則 | 100% |
| 型付きAST | `src/typed_ast.ml:43-45` | `TWhile`, `TFor`, `TLoop` の定義 | 100% |
| 型推論 | `src/type_inference.ml:702-777` | while/for/loop/unsafe/return/defer/assign の型推論 | 100% |
| Core IR脱糖 | `src/core_ir/desugar.ml:366-404` | 簡易実装（Unit値を返す） | 30% |

### 🚧 未完了の実装

| コンポーネント | 実装が必要な内容 | 優先度 | 予定Phase |
|--------------|-----------------|--------|----------|
| Core IR脱糖 | CFGブロックへの展開 | High | Phase 3 |
| CFG構築 | ループブロックの生成 | High | Phase 3 |
| 最適化 | ループ不変式の移動 | Medium | Phase 3-4 |
| LLVM IR生成 | br/phi命令の生成 | High | Phase 3 |
| ランタイム | break/continue サポート | Low | Phase 4 |

## 発見された課題

### 課題1: Core IRの構造的制約

#### 問題の詳細

Core IRは式（`expr`）とブロック（`block`）が明確に分離された設計になっています：

```ocaml
(* src/core_ir/ir.ml *)
type expr = {
  expr_kind : expr_kind;
  expr_ty : ty;
  expr_span : span;
}

type block = {
  label : label;
  params : var_id list;
  stmts : stmt list;
  terminator : terminator;
  block_span : span;
}
```

**制約**:
- `desugar_expr : var_scope_map -> typed_expr -> expr` は単一の式を返す
- ループは複数の基本ブロックで表現されるべき（条件チェックブロック、ボディブロック、出口ブロック）
- `expr_kind` にはループブロックを表現するバリアントが存在しない

**影響**:
- while/for ループを単一の式として脱糖できない
- CFG構築時にループを処理する必要がある
- 関数定義レベル（`fn_def.body: block list`）でないとブロックを生成できない

#### 根本原因

設計意図として、Core IRは「糖衣を剥がした後の正規化された形式」であり、制御フローは基本ブロックとして表現されます。しかし、現在の脱糖パスは式単位で処理を行っており、ブロックを返却できません。

### 課題2: 脱糖パスとCFG構築の責務分担

#### 問題の詳細

現在のコンパイラパイプラインは以下の順序で処理を行います：

```
Typed AST
  ↓ desugar (式を変換)
Core IR (expr)
  ↓ build_cfg_from_expr
CFG (block list)
  ↓ optimize
Optimized CFG
  ↓ codegen
LLVM IR
```

**制約**:
- `desugar` は式レベルの変換のみ（単一exprを返す）
- `build_cfg_from_expr` は既に脱糖された式をブロックに分解
- ループは脱糖時にブロックへ展開すべきだが、脱糖パスでは式しか返せない

**影響**:
- ループを適切なタイミングで処理できない
- 暫定実装として `make_expr (Literal Unit)` を返している（機能しない）

#### 考えられる解決策

**選択肢A: CFG構築時にループを処理**
- 脱糖パスでは「ループマーカー」式を残す
- CFG構築時に専用のループ展開ロジックを実装
- 利点: 既存のアーキテクチャを大きく変更しない
- 欠点: CFGビルダーが複雑化

**選択肢B: 脱糖パスを二段階に分割**
1. 式レベルの脱糖（現在の `desugar_expr`）
2. 制御フロー展開（新規 `expand_control_flow`）
- 利点: 責務が明確
- 欠点: パイプラインの変更が必要

**選択肢C: ブロックビルダーAPIの導入**
- `desugar_expr` がビルダーコンテキストを受け取る
- ループ時にブロックをビルダーに追加
- 利点: 柔軟性が高い
- 欠点: 大規模なリファクタリングが必要

### 課題3: LLVM IR生成での制御フロー表現

#### 問題の詳細

LLVM IRでのループは以下の構造で表現されます：

```llvm
entry:
  %i = alloca i64
  store i64 0, i64* %i
  br label %loop.cond

loop.cond:
  %i.val = load i64, i64* %i
  %cond = icmp slt i64 %i.val, 1000000
  br i1 %cond, label %loop.body, label %loop.exit

loop.body:
  ; ループボディ
  %i.next = add i64 %i.val, 1
  store i64 %i.next, i64* %i
  br label %loop.cond

loop.exit:
  ret void
```

**必要な機能**:
1. ミュータブル変数（`alloca`, `load`, `store`）
2. ラベルとブランチ（`br label`, `br i1`）
3. PHIノード（最適化時）
4. 配列イテレーション（for式用）

**現状**:
- ミュータブル変数は未サポート（参照カウントのみ）
- CFGからLLVM IRへの変換は基本的な制御フローのみ対応
- PHIノードは未生成

## 実装戦略

### Phase 3 での実装計画

#### ステップ1: ミュータブル変数のサポート（Week 25-26）

**目標**: ループカウンタを保持できるようにする

**実装内容**:
1. AST に `mut` キーワードのサポート追加
   ```reml
   let mut i = 0
   ```
2. 型推論で可変変数を追跡
3. Core IR に `Assign` 式を追加（既に stmt には存在）
4. LLVM IR で `alloca`/`load`/`store` を生成

**成果物**:
- `compiler/ocaml/src/ast.ml`: `let_decl` に `is_mutable: bool` フィールド追加
- `compiler/ocaml/src/type_inference.ml`: 可変変数の型チェック
- `compiler/ocaml/src/llvm_gen/codegen.ml`: alloca/load/store 生成

**検証方法**:
```reml
fn test_mut() -> i64 {
  let mut x = 0
  x := 10
  x
}
```

**進捗メモ（2025-10-13）**

- [x] 型環境に `mutability` 情報を追加し、`var` 宣言を `Type_env.extend`/`infer_decl` で処理（`compiler/ocaml/src/type_env.ml`, `compiler/ocaml/src/type_inference.ml`）
- [x] `:=` の型推論で `var` 以外への再代入を拒否し、`ImmutableBinding` / `NotAssignable` 診断を追加（`compiler/ocaml/src/type_error.ml`）
- [x] Core IR で `var_id.vmutable`、`AssignMutable` 式、`Alloca`/`Store` 文を導入し、脱糖パスと CFG 線形化がミュータブル変数をメモリ経由で扱うよう更新（`compiler/ocaml/src/core_ir/*.ml`）
- [x] LLVM コード生成で `Alloca` → `Store` → `Load` のシーケンスを生成し、`Var` 参照時に自動的にロードを挿入（`compiler/ocaml/src/llvm_gen/codegen.ml`）
- [x] 付随パス（定数畳み込み / DCE / モノモルフィゼーション PoC / テストスイート）を新しい IR ノードに対応させ、`desugar` 単体テストを更新

**現状の確認ポイント**

- `let mut` / `:=` の Core IR は `Alloca → Store` に正規化され、再参照は `Load` を経由（SSA への移行準備完了）
- DCE・ConstFold は `Store` を常に副作用アリとして保持するため、ループカウンタが消去されない
- `dune build` は CLI 側の既知 Warning（`warning 21`）で停止するが、新規変更部は型チェックを通過済み

**進捗サマリー（2025-10-13 現在）**

- ✅ `Loop` ノード導入・`desugar` 更新完了（while/for/loop を Core IR へマップ）
- ✅ `linearize_loop` の原型実装完了（ヘッダ φ と latch 差し替え、`value_env` を用いた mini mem2reg）
- ✅ CFG テストに while ケースを追加し、φ 入力と Store を検証
- 🛠 `loop_carried_var` 拡張案・`continue`/`break` 対応設計をドキュメント化（実装は未着手）
- 🛠 For ループ iterator 化の脱糖戦略を整理（`__iter__`/`__next__` 想定）
- ⏳ LLVM IR ゴールデン・統合テスト、ランタイム診断フックは未作業

**進捗サマリー（2025-10-14 更新）**

- ✅ `loop_carried_var` に `lc_sources` を導入し、Core IR (`compiler/ocaml/src/core_ir/ir.ml`)・脱糖パス (`desugar.ml`)・CFG 線形化 (`cfg.ml`) を更新。φ 挿入時に preheader/latch 情報を保持できる状態を整備。
- 📝 IR プリンタ (`ir_printer.ml`) を更新し、`loop_carried` の出力にソース種別を表示してデバッグを容易化。
- ⚠️ `lc_sources` は現状 preheader/latch のみだが、continue 導入時に `LoopSourceContinue` を追加する余地あり。

**次ステップ候補**

- [x] `loop_carried_var` に `lc_sources` を導入し、`continue`/複数更新を取り扱えるよう Core IR 型を拡張
  - 実装済: `compiler/ocaml/src/core_ir/ir.ml`, `compiler/ocaml/src/core_ir/desugar.ml`, `compiler/ocaml/src/core_ir/cfg.ml`, `compiler/ocaml/src/core_ir/ir_printer.ml`
- [x] `cfg.ml` に `continue` ブロック生成・φ 入力増加ロジックを実装し、`test_cfg_continue`（新規）で検証
  - 実装済: `compiler/ocaml/src/core_ir/cfg.ml`（`linearize_loop` が `lc_sources` を解析して preheader/latch/continue の三経路を φ 入力化し、`loop_continue` ブロックを生成）
  - 検証: `compiler/ocaml/tests/test_cfg.ml` に `test_loop_with_continue` を追加し、`loop_continue` ブロックの遷移と φ ノード入力（preheader/latch/continue）の 3 経路を確認済み。
- [x] 型推論でループ外の `continue` をエラー化し、診断コード `E7021` を追加
  - 実装済: `compiler/ocaml/src/type_inference.ml`（ループ深度コンテキストを導入）、`compiler/ocaml/src/type_error.ml`（`ContinueOutsideLoop` 変換）、`compiler/ocaml/tests/test_type_errors.ml`（`test_continue_outside_loop`）
- [x] For ループ iterator モードの脱糖 PoC を実装し、配列版と iterator 版で同じ CFG パスを通ることを確認
  - `compiler/ocaml/src/core_ir/desugar.ml` に `desugar_for_loop` を導入し、配列ソース・イテレータソース双方を `Loop` ノードへ正規化する経路を構築した。
  - `loop_carried` 検出で `for_step` の更新式を確実に拾うよう補強し、Φ ノード経路が配列版でも iterator 版でも一致することを担保。
  - `compiler/ocaml/tests/test_desugar.ml` に `test_desugar_for_loop_cfg_equivalence` を追加し、両モードが生成する CFG の終端シグネチャが同一であることを回帰テストとして固定化。
  - ✅ 配列長取得は Core IR プリミティブ `PrimArrayLength` と LLVM 抽出処理で実装済み（リテラル `0` スタブを解消）。
- [ ] `let mut`/while の LLVM IR ゴールデンテストを追加し、`alloca`/`load`/`store` パターンをスナップショット化
- [ ] ランタイム診断・メトリクスへのフック要否を判断し、必要なら `docs/notes/llvm-spec-status-survey.md` に TODO を追記

**Next Steps**
- [x] 配列ソース向けに実際の長さ取得（`PrimArrayLength` + LLVM 抽出）を実装し、CFG・LLVM の双方でスタブを置き換える。（2025-10-14 完了）
- [ ] `Iterator<T>` 判定を型クラス解決に統合し、`classify_for_source` のヒューリスティックを仕様準拠のトレイト/Capability チェックへ移行する（ステップ3「型クラス統合計画」参照）。
  - ステータス: `constraint_solver.ml` に `IteratorDictInfo` を実装し、`typed_ast.ml` / `core_ir/desugar.ml` が辞書メタデータを受け取る経路を整備済み。`determine_for_source_kind` は `IteratorDictInfo.kind` 依存へ移行し、ヒューリスティック経路を撤廃した。
  - 次のアクション: CLI 出力 (`--emit-typed-ast` / JSON) に `iterator_info` を露出し、辞書メタデータをデバッグできるようにする。Typeclass 戦略書 Section 7 と連動して、Where 句／ユーザー定義 Iterator への拡張計画を整理する。
  - 検証計画: `compiler/ocaml/tests/test_type_errors.ml` に `E7016`（Iterator 制約未満足）のテストを追加済み。今後は JSON ダンプで `effect.stage.*` をスナップショット化し、CLI 統合テストに組み込む。
- [ ] `DictMethodCall` 経由の `has_next` / `next` 呼び出しで Stage / Capability 監査フックを追加し、診断ログ (`effect.stage.*`) との整合を検証する。
  - ステータス: `core_ir/ir.ml` に `iterator_audit` とループ効果リストを追加し、`desugar.ml` / `cfg.ml` で `EffectMarker` を生成する実装を導入。`DictMethodCall` が `audit` を保持し、Stage 要件 (`StageExact「stable」` / `StageAtLeast「beta」`) と Capability ID を記録できる状態になった。
  - 次のアクション: `diagnostic.ml` に Stage/Capability 拡張を出力するヘルパーを追加し、`typeclass.iterator.stage_mismatch` 診断を設計。監査付き IR ゴールデン（`test_desugar` / LLVM スナップショット）を整備し、CI メトリクス `iterator.stage.audit_pass_rate` を登録する。
  - 検証計画: `compiler/ocaml/tests/test_type_errors.ml` で辞書未解決パスを検証済み。今後は `test_desugar`・LLVM ゴールデンテスト・JSON ダンプの追加で監査メタデータを固定化する。

#### ステップ2: CFG構築でのループ展開（Week 26-27）

**目標**: `TWhile` / `TFor` / `TLoop` を Core IR の基本ブロック列へ展開し、SSA 変換に備えたループヘッダ `Phi` 計画を固める。

**現状（2025-10-13）**
- while ループについては `Loop` ノード → CFG → φ ノードまで実装済み。`test_cfg` で φ の入力と Store を検証。
- For ループは IR まで導入済みだが、iterator モードや本格的な更新式合流は未実装。
- `continue`/`break` に関する設計メモを追加済みだが、CFG 生成・テストは未着手。
- LLVM IR ゴールデン・統合テスト、およびランタイム診断の検討はこれから。

**前提整理（2025-10-13 更新）**
- `desugar_expr` がまだループを `Literal Unit` に潰しているため、`cfg.ml` 側ではループ構造を検出できない。
- `core_ir/ir.ml` には `Phi` 文が定義済みだが、`linearize_expr` は if/match 以外で未使用。
- `AssignMutable` は `Alloca`/`Store` ベースで動いており、SSA 化（mem2reg 相当）は未着手。ループカウンタは現在メモリアクセスでのみ表現される。

**実装方針**

**2.1 ループIRマーカーの導入**
- `compiler/ocaml/src/core_ir/ir.ml` に `Loop` 系の `expr_kind` を追加する。`LoopMarker` は以下の情報を保持する想定：
  ```ocaml
  type loop_kind =
    | WhileLoop of expr  (* cond *)
    | ForLoop of for_lowering  (* desugar済みの初期化/更新/イテレータ情報 *)
    | InfiniteLoop

  and loop_info = {
    loop_kind : loop_kind;
    loop_body : expr;
    loop_span : span;
    loop_carried : loop_carried_var list;  (* PHI 候補メタデータ *)
  }
  ```
  - `loop_carried` は `let mut` などループ内で再代入される変数を記録するメタ情報（後述の PHI 挿入で使用）。
  - `for_lowering` には `init`（初期代入列）、`iter_state`（配列長・インデックス変数）、`advance`（更新式）、`pattern`（パターン束縛）を保持する構造体を定義し、`TFor` を while 相当に展開できる粒度まで情報を詰める。
- `compiler/ocaml/src/core_ir/desugar.ml` で `TWhile` / `TFor` / `TLoop` を再帰的に脱糖したサブ式を `LoopMarker` に詰めて返す。既存の `Literal Unit` 返却は廃止する。
  - `loop_carried` の充填は初期段階では空にし、Step2 の後半で静的解析を加える（`collect_loop_carried_mutables` ヘルパを追加）。
  - `for` の場合は、`typed_pattern` から `VarIdGen` を通じた一時変数（インデックス、配列長、現在要素）をここで生成し、CFG 層が追加の糖衣を考慮しなくて済むようにする。

**2.2 CFG ビルダーの拡張**
- `compiler/ocaml/src/core_ir/cfg.ml` に `linearize_loop`（仮名）関数を追加し、`LoopMarker` を検出したら以下のブロック構成を生成する：
  ```
  preheader → header → body → latch ┐
        └─────────────── exit ←─────┘
  ```
  - **preheader**: 既存ブロック（`linearize_expr` 呼出元）を閉じ、`TermJump header` で開始させる。`ForLoop` の初期化式はここで `Assign`/`Store` に展開。
  - **header**: 条件式を評価し、`TermBranch (cond, body_label, exit_label)` を生成。`InfiniteLoop` は `TermJump body_label`。
  - **body**: `loop_body` を通常の式として線形化。`Unit` 結果は `VarIdGen` でダミー変数に束縛。
  - **latch**: `for` の更新式や `continue` 相当の後処理をまとめるブロック。終了時に `TermJump header_label`。
  - **exit**: ループ式の戻り値（現状 Unit）を `TermReturn` か上位ブロックへの `TermJump` に変換。`linearize_expr` から返す結果変数は exit で生成。
- `LabelGen` の連番管理が崩れないよう、`linearize_loop` 内でラベル生成を完結させ、`linearize_expr` は戻り値変数のみを扱う。
- `build_cfg_from_expr` / `build_cfg` の戻り値は既存と同じ `block list`。ループ導入後は `validate_cfg` を活用して基本整形性を都度確認する。
- **2025-10-13 実装メモ**: `cfg_builder` に可変変数の最新 SSA 値を追跡する `value_env` を追加し、preheader で初期値、latch で更新値を回収。ヘッダでは暫定 `Phi` を挿入して `Store` でメモリへ反映し、ループ構築後に latch 側の実値で φ の入力を差し替えることで簡易 mem2reg を実現。これに伴い `collect_loop_carried_vars` で抽出した変数に対し、`set_value_env`/`get_value_env` で初期値と更新値を `Phi` へ渡す。
- **今後の拡張検討（2025-10-13）**
  - **複数更新・`continue` 対応**: 現行の `value_env` は「最後の更新のみ」を latch で拾う実装。`continue` を導入すると latch に合流する経路が増えるため、`loop_carried` を `(var, sources)` 形式へ拡張し、`sources` に `(label, value_expr)` を記録する。`continue` 直前で `value_env` を push し、latch 構築後に `Phi` の入力へ統合する。
  - **`break` 経路の扱い**: `break` は exit 直行のため φ 引数は不要だが、`break` 経路で評価済みの戻り値（将来の非 Unit ループ）を exit ブロックで合流させる仕組みが必要。`exit` ブロックに二段目の φ を設置し、「通常終了」「break」を統合する案を採用する。
  - **診断・テスト**: `continue` を実装した際に φ の入力本数が期待通り増えていることを確認するユニットテスト（`loop_header` ブロックでラベル集合を検証）を追加する。さらに `validate_cfg` に「φ 入力に未解決ラベルが無いか」をチェックするガードを設ける。

**2.3 ループ用 PHI ノード導入方針**
- `loop_carried` に記録された変数について、`header` ブロック先頭に `Phi` 文を挿入する。基本形：
  ```ocaml
  add_stmt builder
    (Phi (var_phi, [ (preheader_label, init_var); (latch_label, updated_var) ]));
  ```
  - `init_var` はループ外で定義された SSA 変数（`let mut` の初期値など）。
  - `updated_var` は `latch` ブロック内で生成する新しい SSA 変数。`AssignMutable` をこの時点で一旦 `Assign` ベースに切り替える必要があるため、Loop 内だけでも `Store` を純 SSA 代入へ昇格させる mini mem2reg を実装する。
- 実装ステップ：
  1. `desugar` 段階で `let mut` の初期値を `loop_carried` に記録する（`VarIdGen` 情報を保持）。
  2. `linearize_loop` で `loop_carried` を参照し、`preheader` / `latch` にそれぞれ `Assign` を挿入する。現状の `Alloca`/`Store` は残しつつ `Phi` を併記し、後段の最適化パスで二重管理を解消する。
  3. `compiler/ocaml/src/core_ir/pipeline.ml` に「ループ内の `Store` を SSA へ昇格させる」軽量パス（仮称 `promote_mutable_in_loops`）を追加し、`Phi` を生成し終えた後に `Store` を削除する計画を明記。
- `break` / `continue` 未実装の間は、`loop_carried` の後辺は常に `latch` 1 箇所に限定されるため、`Phi` の引数は 2 本で固定できる。将来 `continue` が追加される際は、`continue` 先のブロックラベルを `Phi` のソースに追加する設計とする。

**2.4 検証とフォローアップ**
- `./remlc samples/loop/reml --emit-cfg` のようなサンプルを用意し、`loop_cond_X` / `loop_body_X` / `loop_exit_X` の生成を確認。
- `compiler/ocaml/tests/test_cfg_loop.ml`（新規）を追加し、`build_cfg_from_expr` の出力ブロック列をスナップショット化。
- `llvm_ir` ゴールデンに while ループ 1 種を追加し、`Phi` が IR に降りるまでは `alloca` ベースで比較。`Phi` 有効化時点で golden の更新を伴うことをあらかじめ `docs/notes/llvm-spec-status-survey.md` に記載する。
- `loop_carried` 収集の精度を検証するため、`let mut i = 0; while ... { i := i + 1 }` で `Phi` が 1 つだけ生成されること、ネストループでも外側・内側の `Phi` が独立することをユニットテスト化する。
- **2025-10-13 実装メモ**: header で生成する `Phi` は一時的に自身の初期値を2本目にも差し込み、latch の構築後に `loop_latch` からの実値へ差し替える方式に変更。差し替え後は `value_env` を φ の結果に再設定し、ループ脱出後も最新値を共有する。検証として `test_cfg` に φ 入力と Store の有無を確認するケースを追加。
- **計画アップデート**: `loop_carried_var` を将来的に `lc_sources : loop_source list`（`loop_source = { kind : [Preheader|Latch|Continue label]; expr : expr }`）へ拡張し、`Loop` ノードが CFG 構築前に経路情報を保持する設計とする。これにより `continue` のようにヘッダへ戻る複数経路が発生しても、`linearize_loop` 側でラベル整合を取るだけで済む。

**Loop ノード構造案（2025-10-13 試案）**

- `loop_info` を次のように拡張し、SSA 変換前提のメタデータを充実させる：
  ```ocaml
  type loop_source_kind =
    | FromPreheader
    | FromLatch
    | FromContinue of label

  type loop_source = {
    source_kind : loop_source_kind;
    source_span : span;
    source_expr : expr;  (* Preheader の初期値 / Latch の更新式 / continue 前の値 *)
  }

  type loop_carried_var = {
    lc_var : var_id;
    lc_sources : loop_source list;
  }

  type loop_exit_kind =
    | LoopExitNormal
    | LoopExitBreak of label  (* exit へジャンプする break 入口ラベル *)

  type loop_info = {
    loop_kind : loop_kind;
    loop_body : expr;
    loop_span : span;
    loop_carried : loop_carried_var list;
    loop_exits : loop_exit_kind list;  (* break 経路の事前計測用 *)
  }
  ```
  - `lc_sources` に `FromContinue` を含めることで、`continue` の発生地点ごとに φ の入力を追加可能。
  - `loop_exits` は break の発生有無とブロックラベルを保持し、exit ブロック側での φ 合流（通常終了と break 経路の合流）に利用する。

- `desugar` での収集方針：
  1. `AssignMutable` に遭遇したら現在のラベル（`LabelGen` 利用）と式を `FromLatch` 候補として登録。
  2. `continue`（将来実装予定）を特殊式として扱い、直前の可変変数値を `FromContinue` として push、同時に `loop_exits` に break/continue 情報を記録。
  3. `let mut` 初期化は `FromPreheader` として自動生成。

- `linearize_loop` 側での処理：
  - `lc_sources` を走査して `(label, value_var)` の辞書を構築し、φ 挿入時に `builder.blocks` のラベル解決を行う。
  - `FromContinue` の場合、`continue` ターゲット用の中間ブロック（`cont_label -> latch`）を生成し、更新値を φ 入力として接続する。
  - 複数回更新がある場合でも、`lc_sources` の順序に従い deterministic に φ 生成を行える。

**continue を含む CFG サンプル（構想）**

```reml
let mut acc = 0
let mut i = 0
while i < 10 {
  i := i + 1
  if i % 2 == 0 {
    continue
  }
  acc := acc + i
}
```

期待する Core IR+CFG の要点：
- `loop_header` φ: `i_phi = phi [ (preheader, i0); (latch, i_next); (cont_body, i_cont) ]`
- `continue` 用のブロック `loop_continue` を生成し、`acc` など carry 変数の φ 入力にも `loop_continue` 由来の値を追加。
- `acc` φ: `[ (preheader, acc0); (latch, acc_updated); (loop_continue, acc_phi) ]`（`continue` では更新なしのため `acc_phi` は `acc` のヘッダ時点の値）。
- テスト観点で検証するポイント：
  1. ヘッダ φ に `loop_continue` ラベルが入力として追加されている。
  2. `continue` 経路で `acc` が更新されないことを φ の入力値から確認。
  3. `validate_cfg` が未定義ラベルや未接続ブロックを報告しない。

**テストケースの先行整備案**

- `compiler/ocaml/tests/test_cfg_continue.ml`（新規）を用意し、`build_cfg_from_expr` を直接呼び出して φノードの入力集合とブロック遷移を検証する。
  - φノード検証: `loop_header` 内の `Phi` ステートメントを抽出し、`sources` のラベル集合が `{preheader, loop_latch, loop_continue}` になっているかをアサート。
  - `continue` ブロック検証: `loop_continue` の終端が `TermJump loop_latch` になっていること、および carry 変数の更新が存在しないことをチェック。
- `docs/notes/loop-implementation-plan.md` にテスト計画を記載したことで、実装フェーズで迷わずに着手できる。

**成果物**
- `compiler/ocaml/src/core_ir/ir.ml`: `LoopMarker`（正式名称要検討）および補助型追加。
- `compiler/ocaml/src/core_ir/desugar.ml`: ループマーカー生成＋`loop_carried` メタデータ収集。
- `compiler/ocaml/src/core_ir/cfg.ml`: `linearize_loop` 実装、`Phi` 挿入処理、`LabelGen` 管理拡張。
- `compiler/ocaml/src/core_ir/pipeline.ml`: ループ内 `Store` を SSA へ昇格させるフェーズの TODO 追記。
- `docs/notes/llvm-spec-status-survey.md`: ループ SSA 化に伴う LLVM 側の差分とリスクの記録。

**検証方法**
```bash
# 1. CFG スナップショット
remlc samples/loop/simple_while.reml --emit-cfg > /tmp/simple_while.cfg

# 2. ループヘッダに Phi が入っていることを確認
grep -n "Phi" /tmp/simple_while.cfg

# 3. 既存テストの回帰確認
dune runtest compiler/ocaml/tests
```

**リスク・検討事項**
- ループボディ線形化中に `LabelGen` を再帰呼出で共有するため、`linearize_loop` 内でのブロック開始/終了順序を誤ると未閉じブロックが発生する。`builder.current_label` の状態管理をユニットテストでカバーする。
- `loop_carried` の自動検出は Phase 2 の残期間で全パターンを網羅するのが難しいため、初期実装では「ループ直前で `let mut` 宣言された変数のみ」を扱い、他ケースは Warning として診断へ接続する案を採用する。
- LLVM 側で `Phi` と `alloca` が混在すると冗長コードになる。`promote_mutable_in_loops` を遅らせる場合でも、ステップ内で「`alloca` は暫定的に残るが Phase 3 で削除する」旨を技術的負債リストへ追加する。

#### ステップ3: For式の配列イテレーション（Week 27-28）

**目標**: `for x in array { ... }` を動作させる

**実装内容**:

1. インデックス変数の自動生成
2. 配列長の取得（プリミティブ演算追加）
3. パターン変数の束縛
4. **iterator モードへの拡張案**（2025-10-13 更新）
   - `TFor (pat, source, body)` が `Core.Iterable` 互換のイテレータを受け取るケースを想定し、`desugar` 段階で以下の構造を生成する：
     1. `for_init` に `iter_state = source.__iter__()` を追加（必要なら `VarIdGen` で一時変数を作成）。
     2. `for_source` には `iter_state.__next__()` の結果（`Option` / `Result` で返す想定）を保持し、`CFG` では `next_result.is_some()` を条件として評価。
     3. `loop_body` へ入る前に `pattern` を `next_result.unwrap()` から束縛し、`for_step` では追加の更新は不要とする。
  - 配列など固定長コレクションは従来どおりインデックス方式で脱糖し、イテレータを返す型は `for_init`/`for_source` の差し替えだけで共通パスを利用する。これにより Phase 3 で導入予定の `Core.Iterator` と互換性を確保する。

**2025-10-21 追記 — 型クラス統合計画**
- `docs/plans/bootstrap-roadmap/2-1-typeclass-strategy.md` の更新内容と同期し、`for` ループで `Iterator` トレイト辞書を要求する方針を明文化する。`type_inference.ml` では `TFor` 構築時に `TraitConstraint`（仮称）を発行し、解決済み辞書を `typed_ast` メタデータとして保持する。【参照: docs/spec/3-1-core-prelude-iteration.md】
- `constraint_solver.ml` に `solve_iterator` を追加し、`Array<T>` / `Slice<T>` / `Core.Iter.Iter<T>` / `Option<T>` など仕様でイテレータ提供が保証されている型を暗黙辞書 `DictImplicit ("Iterator", [source_ty; item_ty])` として返す。辞書レイアウトには `has_next` / `next` / `size_hint` メソッドを登録し、Stage 情報 (`effects.contract.stage_mismatch`) を `DictConstruct.metadata` に書き込む。【参照: docs/spec/1-2-types-Inference.md, docs/spec/3-6-core-diagnostics-audit.md】
- `typed_ast` の `TFor` に `iterator_dict : dict_ref` を追加し、`desugar_for_loop` では `classify_for_source` を廃止して辞書参照を直接利用する。辞書が得られなかった場合は型クラス診断（仮称 `typeclass.iterator.unsatisfied`）を発生させ、ヒューリスティック経路にフォールバックしない。
- `DictMethodCall` には Stage / Capability 情報を拡張項目として付与し、`docs/spec/3-8-core-runtime-capability.md` の `StageRequirement::{Exact, AtLeast}` と照合できるようにする。監査ログキーは `effect.stage.iterator` を予定し、`docs/spec/3-6-core-diagnostics-audit.md` への追記を別タスクとして登録する。

**データフローの見直し**
1. 型推論 (`infer_expr`) が `Iterator` 制約を生成し、`Constraint_solver.solve` から辞書参照を受け取る。
2. `type_env` / `typed_ast` が辞書参照を保持し、`desugar` へ `iterator_dict` を引き渡す。
3. `desugar_for_loop` が辞書から `has_next` / `next` / `size_hint` のインデックスを解決し、`DictMethodCall` ノードを生成する。
4. LLVM 生成では `trait_method_indices "Iterator"` を導入し、辞書の vtable から該当メソッドポインタをロードする。

**検証計画**
- `compiler/ocaml/tests/test_type_errors.ml` に「`Iterator` 未実装型を `for` に渡すと診断が出る」テストケースを追加。
- `compiler/ocaml/tests/test_desugar.ml` に辞書経路の for ループ脱糖スナップショットを追加し、`classify_for_source` 撤廃後も CFG が一致することを確認。
- `compiler/ocaml/tests/llvm-ir/golden` に `for_iterator.ll.golden`（新規）を用意し、間接呼び出しが辞書経由になっていることをゴールデンテストで固定化する。

**リスクとフォローアップ**
- `Iterator` トレイトの正式名称・関連型が仕様ドラフト段階のため、`constraint_solver` では `Core.Iter.Iterator` プレフィックスを使用し、名称変動時に差分が局所化されるようにする。
- `typed_ast` へ辞書情報を追加すると CLI の `--emit-typed-ast` 出力形式が変わるため、併せて `typed_ast_printer.ml` の更新と互換性メモを作成する。
- Stage 情報を辞書へ付与する際に Capability Registry の ID が未確定な場合は TODO コメントで追跡し、`docs/spec/3-6-core-diagnostics-audit.md` の更新タスクとリンクさせる。

```ocaml
(* desugar.ml での for式処理 *)
| TFor (pat, source, body, iterator_dict, iterator_info) ->
    let source_expr = desugar_expr map source in
    let body_expr = desugar_expr map body in

    (* インデックス変数 *)
    let index_var = VarIdGen.fresh "__for_index" ty_i64 span in

    (* パターン変数抽出 *)
    let pat_var = extract_pattern_var pat in

    (* 初期化: index = 0 *)
    let init = [(index_var, make_expr (Literal (Int ("0", Base10))) ty_i64 span)] in

    (* 条件: index < array.length *)
    let index_ref = make_expr (Var index_var) ty_i64 span in
    let array_len = make_expr (ArrayLength source_expr) ty_i64 span in
    let cond = make_expr (Primitive (PrimLt, [index_ref; array_len])) ty_bool span in

    (* 更新: index = index + 1 *)
    let one = make_expr (Literal (Int ("1", Base10))) ty_i64 span in
    let index_incr = make_expr (Primitive (PrimAdd, [index_ref; one])) ty_i64 span in
    let update = [(index_var, index_incr)] in

    (* ボディ: let pat_var = array[index] in body *)
    let array_access = make_expr (ArrayAccess (source_expr, index_ref)) pat.tpat_ty span in
    let body_with_binding = make_expr (Let (pat_var, array_access, body_expr)) ty span in

    make_expr
      (LoopMarker {
        loop_kind = ForLoop (pat, source);
        loop_cond = Some cond;
        loop_init = init;
        loop_update = update;
        loop_body = body_with_binding;
      })
      ty span
```

**必要な追加機能**:
- `ArrayLength` 式の追加（`expr_kind` に追加）
- LLVM IR での配列長取得（構造体の先頭フィールド）

**成果物**:
- `compiler/ocaml/src/core_ir/ir.ml`: `ArrayLength` 式追加
- `compiler/ocaml/src/llvm_gen/codegen.ml`: 配列長の取得実装

**検証方法**:
```reml
fn test_for() -> i64 {
  let arr = [1, 2, 3, 4, 5]
  let mut sum = 0
  for x in arr {
    sum := sum + x
  }
  sum  // 15
}
```

#### ステップ4: LLVM IR生成の完成（Week 28-29）

**目標**: CFGをLLVM IRのbr/phi命令に変換

**実装内容**:

**4.1 基本ブロックのラベル生成**

```ocaml
(* codegen.ml *)
let codegen_block (llctx : llcontext) (llmod : llmodule) (llfn : llvalue)
    (block : block) (block_map : (label, llbasicblock) Hashtbl.t) : unit =

  (* ラベルに対応するLLVMブロックを取得または作成 *)
  let llblock =
    match Hashtbl.find_opt block_map block.label with
    | Some bb -> bb
    | None ->
        let bb = append_block llctx block.label llfn in
        Hashtbl.add block_map block.label bb;
        bb
  in

  position_at_end llblock builder;

  (* ブロック内の命令を生成 *)
  List.iter (codegen_stmt llctx llmod builder) block.stmts;

  (* 終端命令を生成 *)
  codegen_terminator llctx llmod builder block.terminator block_map

and codegen_terminator (llctx : llcontext) (llmod : llmodule) (builder : llbuilder)
    (term : terminator) (block_map : (label, llbasicblock) Hashtbl.t) : unit =

  match term with
  | TermReturn expr ->
      let llval = codegen_expr llctx llmod builder expr in
      ignore (build_ret llval builder)

  | TermJump target_label ->
      let target_block = Hashtbl.find block_map target_label in
      ignore (build_br target_block builder)

  | TermBranch (cond_expr, then_label, else_label) ->
      let llcond = codegen_expr llctx llmod builder cond_expr in
      let then_block = Hashtbl.find block_map then_label in
      let else_block = Hashtbl.find block_map else_label in
      ignore (build_cond_br llcond then_block else_block builder)

  | TermSwitch (scrutinee, cases, default_label) ->
      let llscrutinee = codegen_expr llctx llmod builder scrutinee in
      let default_block = Hashtbl.find block_map default_label in
      let llswitch = build_switch llscrutinee default_block (List.length cases) builder in
      List.iter (fun (lit, case_label) ->
        let llcase_val = codegen_literal lit in
        let case_block = Hashtbl.find block_map case_label in
        add_case llswitch llcase_val case_block
      ) cases

  | TermUnreachable ->
      ignore (build_unreachable builder)
```

**4.2 関数全体のCFG処理**

```ocaml
(* codegen.ml *)
let codegen_function (llctx : llcontext) (llmod : llmodule) (fn_def : fn_def) : llvalue =
  (* 関数シグネチャを生成 *)
  let llfn = declare_function fn_def.fn_name fn_ty llmod in

  (* ブロックマップを作成（ラベル → LLVMブロック） *)
  let block_map = Hashtbl.create 16 in

  (* エントリブロックを作成 *)
  let entry_block = append_block llctx "entry" llfn in
  Hashtbl.add block_map "entry" entry_block;

  (* 全ブロックのラベルを事前登録（前方参照対応） *)
  List.iter (fun block ->
    let llblock = append_block llctx block.label llfn in
    Hashtbl.add block_map block.label llblock
  ) fn_def.body;

  (* 各ブロックのコード生成 *)
  List.iter (fun block ->
    codegen_block llctx llmod llfn block block_map
  ) fn_def.body;

  llfn
```

**成果物**:
- `compiler/ocaml/src/llvm_gen/codegen.ml`: ブロック単位のコード生成
- `compiler/ocaml/tests/llvm-ir/golden/while_loop.ll.golden`: ゴールデンテスト

**検証方法**:
```bash
# LLVM IR を生成
./remlc test.reml --emit-ir

# 生成されたIRを確認
cat test.ll

# llc でネイティブコードに変換
llc test.ll -o test.s

# 実行可能ファイルを生成
gcc test.s runtime.a -o test
./test
```

#### ステップ5: 統合テストとベンチマーク（Week 29-30）

**目標**: 型クラスベンチマークを実行可能にする

**実装内容**:

1. while/forループの統合テスト作成
2. ベンチマークコードの動作確認
3. 型クラスベンチマークスクリプトの実行

**テストケース**:

```reml
// tests/integration/test_while_loop.reml
fn test_simple_while() -> i64 {
  let mut i = 0
  let mut sum = 0
  while i < 10 {
    sum := sum + i
    i := i + 1
  }
  sum  // 45
}

fn test_for_array() -> i64 {
  let arr = [1, 2, 3, 4, 5]
  let mut sum = 0
  for x in arr {
    sum := sum + x
  }
  sum  // 15
}

fn test_nested_loop() -> i64 {
  let mut sum = 0
  let mut i = 0
  while i < 10 {
    let mut j = 0
    while j < 10 {
      sum := sum + 1
      j := j + 1
    }
    i := i + 1
  }
  sum  // 100
}
```

**ベンチマーク実行**:

```bash
# ベンチマークスクリプトを実行
cd compiler/ocaml
./scripts/benchmark_typeclass.sh

# 結果を確認
cat benchmark_results/comparison_report.md
```

**成果物**:
- `compiler/ocaml/tests/integration/test_loops.reml`: 統合テスト
- `compiler/ocaml/benchmarks/*.reml`: 動作確認済みベンチマーク
- `docs/notes/typeclass-performance-evaluation.md`: 計測結果

### Phase 4 での拡張機能

#### break/continue サポート

**実装方針**:
- `break` → 出口ブロックへの直接ジャンプ
- `continue` → 条件チェックブロックへの直接ジャンプ
- ネストしたループでのスコープ管理

#### ループ最適化

**最適化項目**:
1. ループ不変式の移動（Loop-Invariant Code Motion）
2. ループ融合（Loop Fusion）
3. ループ展開（Loop Unrolling）
4. 強度削減（Strength Reduction）

## 技術的な詳細

### ミュータブル変数の実装詳細

#### 型システムへの影響

```ocaml
(* types.ml *)
type mutability = Immutable | Mutable

type var_info = {
  var_name : string;
  var_ty : ty;
  var_mutability : mutability;
  var_span : span;
}
```

#### 型チェック

```ocaml
(* type_inference.ml *)
let check_assignment (env : env) (lhs : expr) (rhs : expr) : (unit, type_error) result =
  match lhs.expr_kind with
  | Var id ->
      (match lookup_var_info env id.name with
      | Some var_info when var_info.var_mutability = Mutable ->
          Ok ()
      | Some var_info ->
          Error (ImmutableAssignment (id.name, lhs.expr_span))
      | None ->
          Error (UnboundVariable (id.name, lhs.expr_span)))
  | _ ->
      Error (InvalidLValue lhs.expr_span)
```

### CFG構築の詳細

#### ブロック結合アルゴリズム

```ocaml
(* cfg.ml *)
let patch_block_terminator (block : block) (new_term : terminator) : block =
  { block with terminator = new_term }

let connect_blocks (pred_block : block) (succ_label : label) : block =
  match pred_block.terminator with
  | TermReturn _ ->
      (* 既にreturnがある場合は変更しない *)
      pred_block
  | TermJump _ | TermBranch _ | TermSwitch _ | TermUnreachable ->
      (* 既にジャンプがある場合も変更しない *)
      pred_block
  | _ ->
      (* 終端命令がない場合、ジャンプを追加 *)
      patch_block_terminator pred_block (TermJump succ_label)
```

#### ドミネータ解析（将来の最適化用）

```ocaml
(* cfg.ml *)
type dominator_tree = {
  idom : (label, label option) Hashtbl.t;  (* 即座ドミネータ *)
  dominated : (label, label list) Hashtbl.t;  (* 支配されるブロック *)
}

let compute_dominators (blocks : block list) (entry_label : label) : dominator_tree =
  (* Lengauer-Tarjan アルゴリズム *)
  ...
```

### LLVM IR生成の詳細

#### alloca/load/store パターン

```llvm
; ミュータブル変数のalloca（関数エントリで生成）
entry:
  %i.addr = alloca i64
  store i64 0, i64* %i.addr
  br label %loop.cond

; ループ内でのload/store
loop.body:
  %i.val = load i64, i64* %i.addr
  %i.next = add i64 %i.val, 1
  store i64 %i.next, i64* %i.addr
  br label %loop.cond
```

#### PHIノード生成（最適化版）

```llvm
; SSA形式でのループカウンタ
loop.cond:
  %i = phi i64 [ 0, %entry ], [ %i.next, %loop.body ]
  %cond = icmp slt i64 %i, 1000000
  br i1 %cond, label %loop.body, label %loop.exit

loop.body:
  ; ボディ処理
  %i.next = add i64 %i, 1
  br label %loop.cond
```

## テスト戦略

### 単体テスト

| テスト対象 | ファイル | テスト内容 |
|----------|---------|----------|
| ミュータブル変数 | `test_mut_var.ml` | let mut、代入、型チェック |
| Whileループ | `test_while.ml` | 基本while、ネストwhile、早期脱出 |
| Forループ | `test_for.ml` | 配列イテレーション、範囲、パターン |
| CFG構築 | `test_cfg_loops.ml` | ブロック生成、ジャンプ先、ドミネータ |
| LLVM IR | `test_codegen_loops.ml` | br命令、phi命令、最適化 |

### 統合テスト

```reml
// tests/integration/comprehensive_loops.reml

// フィボナッチ数列
fn fibonacci(n: i64) -> i64 {
  if n <= 1 { return n }

  let mut a = 0
  let mut b = 1
  let mut i = 2

  while i <= n {
    let tmp = a + b
    a := b
    b := tmp
    i := i + 1
  }

  b
}

// 配列の合計
fn array_sum(arr: [i64]) -> i64 {
  let mut sum = 0
  for x in arr {
    sum := sum + x
  }
  sum
}

// 二重ループ
fn matrix_sum(rows: i64, cols: i64) -> i64 {
  let mut sum = 0
  let mut i = 0

  while i < rows {
    let mut j = 0
    while j < cols {
      sum := sum + (i * cols + j)
      j := j + 1
    }
    i := i + 1
  }

  sum
}
```

### ベンチマーク

```bash
# 型クラスベンチマークの実行
./scripts/benchmark_typeclass.sh

# 期待される出力:
# ========================================
# Reml 型クラス実装ベンチマーク
# ========================================
#
# [マイクロベンチマーク]
# - bench_eq_i64:     実行完了 (10^6 回)
# - bench_eq_string:  実行完了 (10^6 回)
# - bench_ord_i64:    実行完了 (10^6 回)
#
# [マクロベンチマーク]
# - find_element:     実行完了
# - bubble_sort:      実行完了
# - count_in_range:   実行完了
#
# 詳細レポート: benchmark_results/comparison_report.md
```

## 既知の制約と回避策

### 制約1: 配列の実装が不完全

**制約**: 現時点で配列リテラル `[1, 2, 3]` の型推論は未実装

**回避策**: Phase 3で配列型を完全実装してからforループを実装

**技術的負債**: `docs/compiler/ocaml/docs/technical-debt.md` の M1 に記録済み

### 制約2: クロージャでのキャプチャ

**制約**: ループ内でクロージャを生成する場合、ループ変数のキャプチャが未対応

**回避策**: Phase 3ではクロージャ内でのループ変数使用を制限

**将来の実装**: Phase 4でクロージャ変換を完全実装

### 制約3: break/continueの欠如

**制約**: 現時点ではbreak/continueをサポートしない

**回避策**: フラグ変数を使った早期脱出パターンを推奨

```reml
// break の代替パターン
let mut found = false
let mut i = 0
while i < n && !found {
  if condition {
    found := true
  }
  i := i + 1
}
```

## 成功基準

### Phase 3 完了時の基準

- [ ] ミュータブル変数の宣言・代入が動作
- [ ] Whileループが正しくコンパイルされてLLVM IRを生成
- [ ] Forループが配列をイテレートできる
- [ ] ネストしたループが正しく動作
- [ ] ベンチマークスクリプトが完走する
- [ ] 型クラス性能評価レポートが完成
- [ ] 全既存テストが引き続き成功（レグレッションゼロ）

### Phase 4 完了時の基準

- [ ] break/continueが実装されている
- [ ] ループ最適化が有効化されている
- [ ] PHIノードが生成されている
- [ ] クロージャ内でループ変数をキャプチャできる
- [ ] 配列以外のイテレータ（Range、Customなど）をサポート

## 参考資料

### 内部資料

- [Bootstrap Roadmap Phase 3](../plans/bootstrap-roadmap/3-0-phase3-overview.md)
- [型クラス戦略](../plans/bootstrap-roadmap/2-1-typeclass-strategy.md)
- [Core IR設計](../plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md)
- [LLVM統合ガイド](../guides/llvm-integration-notes.md)
- [技術的負債リスト](../../compiler/ocaml/docs/technical-debt.md)

### 外部資料

- [LLVM Language Reference Manual - Terminator Instructions](https://llvm.org/docs/LangRef.html#terminator-instructions)
- [SSA Book - Chapter 3: Control Flow Graphs](http://ssabook.gforge.inria.fr/latest/book.pdf)
- [Modern Compiler Implementation in ML - Chapter 7: Activation Records](https://www.cs.princeton.edu/~appel/modern/ml/)

## 更新履歴

- **2025-10-13**: 初版作成（Phase 2 Week 20-21 でのループ実装試行の結果を記録）
