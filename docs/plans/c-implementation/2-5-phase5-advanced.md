# フェーズ 5: 高度な機能と仕様準拠

このフェーズでは、Reml の複雑な機能であるパターンマッチング、Algebraic Effects (代数的効果)、および任意精度演算に取り組みます。

## 5.0 前提と範囲
- **前提**: フェーズ 4 までのコード生成・基本型・AST/型検査の基盤が動作していること。
- **対象**: BigInt / パターンマッチ / 文字列と Unicode / ADT・レコード / 参照型 / トレイト・型クラス / 効果行 / 型推論の拡張 / Effect の初期ランタイム。
- **成果物**: 仕様準拠の型・効果・文字列処理と、主要な高機能構文が C 実装で動作すること。
- **非対象**: すべての最適化・完全な効果システムの最終形・全プラグイン連携・GC。

## 5.1 BigInt 統合
- **ライブラリ**: `libtommath` (必要に応じて `gmp` ラッパー)。
- **仕様参照**: `docs/spec/1-2-types-Inference.md`、`docs/spec/3-4-core-numeric-time.md`。
- **タスク**:
  1.  `deps/` へのライブラリ統合。
  2.  `Core.Numeric` プリミティブバインディングの実装（加減乗除・比較・符号）。
  3.  大きな整数リテラルの解析サポート (例: `12345678901234567890`) とリテラル種別の確定。
  4.  `Int` (64-bit) のオーバーフロー時の扱いを明文化（`BigInt` 昇格 / 例外 / 診断）。
  5.  文字列変換・表示・パース（`to_string` / `parse`）の基本 API。
  6.  診断 ID とエラーメッセージの整備（桁あふれ/無効リテラル）。
  7.  テスト: 算術演算、境界値、文字列変換、リテラル解析。

### 5.1.1 実装方針（C 実装）
- **バックエンド**: 既定は `libtommath`。`gmp` は性能要件が明確になった段階で `CMake` オプションに切り替えられるようにする。
- **API 形状**: `reml_bigint` を C 側のラッパー構造体として定義し、`mp_int` の初期化/破棄/演算/比較/符号判定を隠蔽する。
- **リテラル種別**: `REML_LITERAL_BIGINT` を追加し、`REML_LITERAL_INT` のパース時に `i64` へ収まらない場合は `BigInt` へ昇格する。
- **型推論**: `BigInt` は `Int` と別のプリミティブ型として扱う。数値演算は `Int`/`Float`/`BigInt` のうち **同一型の演算**のみ許可し、必要な昇格は後続の `5.9` で統一ルールを定める。
- **オーバーフロー**:
  - **リテラル**: `Int` へ収まらない場合は `BigInt` へ昇格。ただし期待型が明示的に `Int` の場合は `parser.number.overflow` 診断を出す。
  - **演算**: `Int` のランタイムオーバーフローは `5.9` での統一ルール確定まで `panic`/`wrap` の方針を保留し、C 実装では診断を出せる足場だけ先行する。

### 5.1.2 作業ステップ（詳細）
- [x] `compiler/c/cmake/FetchDependencies.cmake` に `libtommath` のビルドターゲットを追加し、`reml_core` からリンクできるようにする。
- [x] `compiler/c/include/reml/numeric/bigint.h` と `compiler/c/src/numeric/bigint.c` を追加し、`reml_bigint` API を定義する。
- [x] `Core.Numeric` との接続レイヤー（`reml_numeric_bigint_*`）を用意し、演算/比較/符号/変換関数を公開する。
- [x] `compiler/c/include/reml/ast/ast.h` の `reml_literal_kind` に `REML_LITERAL_BIGINT` を追加し、パーサーで昇格判定を行う。
- [x] `compiler/c/include/reml/typeck/type.h` に `REML_TYPE_BIGINT` を追加し、型推論と `numeric` 判定を拡張する。
- [x] `compiler/c/include/reml/sema/diagnostic.h` に数値リテラル関連の診断コードを追加し、`parser.number.overflow`/`parser.number.invalid` に対応する。
- [x] `compiler/c/src/codegen/codegen.c` で `BigInt` リテラルと演算をランタイム呼び出しへ下降させる（MVP では演算子 `+ - * / %` のみ）。

### 5.1.3 診断とエラーメッセージ方針
- **桁あふれ**: `parser.number.overflow`（`E7101`）を使用し、`Int` 期待型の文脈で `BigInt` 昇格ができない場合に発火する。
- **無効リテラル**: `parser.number.invalid`（`E7102`）を使用し、基数/桁区切りの不正を報告する。
- **メタデータ**: `radix`/`min`/`max`/`repr` を診断拡張として保持し、IDE で補助表示できるようにする。

### 5.1.4 テスト計画（MVP）
- **演算**: `add/sub/mul/div/rem` の正確性（正負/符号付き）、比較 (`< <= == >= >`) の境界。
- **リテラル**: 10 進/2 進/8 進/16 進、`_` 区切り、`i64` 境界直前/直後の昇格。
- **文字列変換**: `to_string`/`parse` の往復、先頭 `+`/`-`、空文字/不正文字の診断。
- **型推論**: `Int`/`BigInt` の混在で明示注釈を要求するケースを追加。

### 5.1.5 進捗メモ（2026-01-02）
- `libtommath` を `reml_tommath` として静的リンクする経路を追加済み。
- `BigInt` リテラルの昇格判定と AST/型の追加を完了。
- `bigint` API ラッパーを追加済み（演算/比較/変換）。`Core.Numeric` 接続を実装済み。
- `bigint` リテラル/演算/比較をランタイム呼び出しへ降下済み。

## 5.2 パターンマッチングのコンパイル
- **目標**: 効率的な決定木 (decision tree) 生成。
- **仕様**: `docs/plans/pattern-matching-improvement/`。
- **仕様参照**: `docs/spec/1-5-formal-grammar-bnf.md`（構文）、`docs/spec/1-2-types-Inference.md`（パターンの型制約）。
- **採用方針（決定）**:
  - **アルゴリズム**: パターン行列から決定木を構築する方式（Maranget 系の列選択 + 分割）。
  - **参照計画**: `docs/plans/pattern-matching-improvement/1-2-match-ir-lowering-plan.md` と `docs/plans/pattern-matching-improvement/1-1-pattern-surface-plan.md` を基準とする。
- **アプローチ**:
  - `match` 式を `switch` と `if` チェックの列にコンパイルする。
  - ヒューリスティック: 判別式 (Enums/ADT) を優先し `switch` 化、Range/Slice/Or は行列分割で段階的に展開、ガードは最後に評価。
- **タスク**:
  1.  `DecisionTree` ビルダーの実装。
  2.  網羅性と冗長性のチェック（診断 ID の確定）。
  3.  ガードやネストパターンの優先順位ルールを固定。
  4.  Codegen: `DecisionTree` を LLVM IR (BasicBlocks, Br) に下降させる。
  5.  テスト: Enum/整数/タプル/リテラル/ガードの組み合わせ。

### 5.2.1 進捗メモ（2026-01-03）
- リテラル/ワイルドカード/識別子パターンを対象に、直列分岐の DecisionTree と LLVM IR への降下を追加。
- bool の網羅性/到達不能診断（`REML_DIAG_PATTERN_EXHAUSTIVENESS_MISSING` / `REML_DIAG_PATTERN_UNREACHABLE_ARM`）を Sema に追加。
- C 単体テストに match の成功/失敗ケースを追加。
- int/bool のリテラルパターンに対する `switch` 化ヒューリスティックを Codegen に追加。
- ADT/Enum への入口としてコンストラクタパターンの AST/パーサー表現を追加（セマンティクス/コード生成は未対応）。

### 5.2.2 進捗メモ（2026-01-03）
- `when` ガード構文を lexer/parser に追加し、match arm にガード式を保持。
- Range パターン（`..` / `..=`）の AST/パーサー受理と型検査を追加。
- Enum タグを `i32` として扱う最小表現を typeck/codegen に追加し、コンストラクタパターンでタグ比較を行う分岐を実装。
- `switch` ヒューリスティックを Range/Enum へ拡張（小さな範囲とゼロ引数コンストラクタのみ）。

### 5.2.3 進捗メモ（2026-01-03）
- Enum 表現を `{ tag: i32, payload: *i8 }` とし、ペイロードの GEP/ロードでフィールド束縛と比較を実装。
- ガード付きの match でも `switch` 分岐後に guard を評価し、失敗時は残りアームを再評価する経路を追加。
- Range/Enum の到達不能判定を拡張し、重複レンジ/タグを検出。

### 5.2.4 進捗メモ（2026-01-03）
- Enum/ADT のコンストラクタ式を追加し、ランタイムで値生成を行う経路を実装。
- Range の網羅性判定で区間被覆（`INT64_MIN..INT64_MAX`）をチェックするよう精緻化。
- guard 付き `switch` の fallback を共有化し、再評価分岐の重複を削減。

### 5.2.5 設計メモ（2026-01-03）
- Enum/ADT ペイロードの破棄は **式スコープ単位** を基本にし、`let` の束縛スコープ終了時・`return`・ブロック末尾で `reml_enum_free` を挿入する。
- 破棄は **所有権が確定している値のみ** に限定し、共有参照や引数渡しは別途 `retain`/`clone` が必要になった時点で導入する。

### 5.2.6 実装済み範囲（チェック）
- [x] リテラル/ワイルドカード/識別子パターンの decision tree 生成（直列分岐）
- [x] bool リテラルの網羅性/冗長性チェック
- [x] LLVM IR への分岐降下（`br` ベース）
- [x] int/bool リテラルパターンの `switch` 生成
- [x] C 単体テストの追加（sema/codegen）
- [x] コンストラクタパターンの AST/パーサー受理
- [x] Enum タグ (`i32`) を使ったコンストラクタ分岐（ゼロ引数のみ）
- [x] ガード構文の受理と match 分岐への組み込み
- [x] Range パターンの受理と分岐展開
- [x] ADT/Enum のペイロード分岐（フィールドの束縛・型検査・GEP/ロード）
- [x] ADT/Enum のコンストラクタ式とランタイム生成
- [x] Enum/ADT ペイロード破棄の設計方針を確定
- [x] `reml_enum_free` の挿入（スコープ終端/return/ブロック末尾）
- [x] タプル/レコードの分岐展開（AST/Type/Sema/Codegen + テスト）
- [x] `pattern.exhaustiveness.missing` の `extensions.pattern` 生成（missing_variants/missing_ranges）
- [x] enum の drop 経路を含む IR 生成テストを追加
- [x] ガード/ネストパターンの優先順位ルール固定（仕様のみ）
- [x] `switch` 化のヒューリスティック適用（Int/Bool/Range/Enum）

### 5.2.7 優先順位ルール（確定）
- **評価順**: 外側パターン → 内側パターン（左から順に束縛）→ ガード式 → アーム本体。
- **ガードの位置付け**: パターンが成功した後にのみ評価し、失敗時は次のアームへ進む。
- **ネスト展開の基準**: ADT/Enum の場合はタグ判定を最優先し、タグ一致後にフィールド/サブパターンを評価する。
- **束縛の可視性**: ガード/本体からはパターン束縛が参照可能。ガード失敗時に束縛は破棄される。

### 5.2.8 `switch` 化ヒューリスティック（確定）
- 対象: scrutinee が `Int`/`Bool` で、アームが **リテラルのみ + 末尾の catch-all** の場合に `switch` を生成する。
- 追加: **小さな Range**（上限 8 個まで）と **ゼロ引数の Enum コンストラクタ** は `switch` へ展開する。
- 除外: ガードやネスト（タプル/レコード/コンストラクタ payload）が含まれる場合は直列分岐へフォールバック。
- 例外: `Bool` で `true/false` が揃う場合は catch-all 無しでも `switch` を許可する。

### 5.2.9 残タスクと推奨順
1. **DecisionTree の最適化**: 列選択/分割（Maranget 系）を本格化し、ネスト/Or/Range の行列分割を整理。
2. **網羅性/到達不能の拡張**: タプル/レコード/コンストラクタ payload を含むケースへ診断を拡張。
3. **診断の JSON 出力拡張**: `extensions.pattern` を CLI/LSP 出力に反映する。

### 5.2.10 診断 JSON 出力と LSP 連携の設計メモ（2026-01-03）
- **CLI 入口**: `reml internal codegen --diag-json <file>` を追加し、診断を JSON で標準出力へ出す。
- **出力形**: `{ "sema": [...], "codegen": [...] }` の 2 本立て。各要素は
  - `code` (int), `message` (string), `span` (start/end line/column)
  - `extensions.pattern` に `missing_variants` / `missing_ranges` を収容
- **LSP 連携**: 将来的に `--diag-json` の出力を LSP サーバへパイプし、`Diagnostic.data` に `extensions` を付与する。
  - `pattern.exhaustiveness.missing` は `data.extensions.pattern` を保持し、IDE 側で不足パターンを表示する。
  - `pattern.range.bound_inverted` 等の拡張も同様に `extensions.pattern` へ集約する。

### 5.2.11 進捗メモ（2026-01-03）
- Enum コンストラクタの `switch` 生成で、**同一 tag の複数アームを 1 ケースへ統合**する最適化を追加。
- `--diag-json` の JSON 形式と LSP 接続の設計メモを追記。

### 5.2.12 残余パターン抽出の詳細設計（2026-01-03）
- **目的**: `pattern.exhaustiveness.missing` の精度を上げ、タプル/レコード/コンストラクタ payload の不足箇所を `extensions.pattern` に構造化して出力する。
- **表現案**: `extensions.pattern.missing` を導入し、`missing_variants`/`missing_ranges` と併用。
  - `missing_variants`: `["Some", "None"]` のような enum 直下の不足コンストラクタ。
  - `missing_ranges`: `{start,end,inclusive}` の配列（Int の未被覆区間）。
  - `missing_tuples`: `[{ arity: 2, items: [PatternMissing, PatternMissing] }]`
  - `missing_records`: `[{ fields: [{ name: "x", missing: PatternMissing }, ...] }]`
  - `missing_payloads`: `[{ ctor: "Some", missing: PatternMissing }]`
- **PatternMissing の構造**（再帰型）:
  - `any`（ワイルドカード相当）
  - `literal`（`int/float/bool/...`）
  - `range`（`start/end/inclusive`）
  - `tuple`（`arity` + `items`）
  - `record`（`fields` 配列）
  - `constructor`（`ctor` + `payload`）
- **抽出アルゴリズム（概要）**:
  1.  **Maranget の行列分割**で列を選択し、各列の**残余**を計算する。
  2.  **constructor**: 同一コンストラクタの行を束ね、payload の残余を再帰的に算出。
  3.  **tuple/record**: 先頭列で分割後、残余列の**直積合成**で `missing_tuples/records` を生成。
  4.  **guard**: guard 付きアームは「網羅性寄与しない」扱いとし、残余抽出対象から除外。
  5.  **縮約**: 隣接区間のマージ、重複パターンの除去、`any` の吸収を行う。
- **段階導入**:
  - Phase A: enum payload の不足のみ `missing_payloads` へ出力。
  - Phase B: tuple の `missing_tuples` を追加。
  - Phase C: record の `missing_records` と field 単位の不足を追加。

## 5.3 Algebraic Effects (ランタイムサポート)
- **目標**: `perform`, `resume`, `handle` のサポート。
- **仕様参照**: `docs/spec/1-3-effects-safety.md`、`docs/spec/3-8-core-runtime-capability.md`。
- **戦略（決定）**:
  - **CPS 変換 + ステートマシン化**を採用（LLVM の `switch`/`br` でジャンプテーブル生成）。
  - **非ポータブル API (`makecontext`/`swapcontext`) とスタックコピー方式は Phase 5 では採用しない**。
  - State/Reader などの単純な効果は **Typed State Passing** の糖衣として扱い、CPS 変換の同一ランタイム基盤に合流させる。
- **タスク**:
  1.  効果ハンドラの最小ランタイム API 定義（C ABI、リソース解放規約、one-shot 保証）。
  2.  `perform` / `resume` / `handle` の MIR/IR ノード定義と CPS 変換パスの追加。
  3.  生成されるステートマシンのランタイム実行器（trampoline）実装。
  4.  `resume` の再入禁止（one-shot）検査と診断を実装。
  5.  例外/State を代表ケースとして実装し、テストで動作保証。

### 5.3.1 実装方針（C 実装）
- **効果境界**: `perform` は最寄りの `handle` へ制御を移す。ハンドラが無い場合は `effects.unhandled` として停止。
- **one-shot 保証**: 継続 (`continuation`) は 1 度だけ `resume` 可能。2 回目以降は診断を発行して停止。
- **CPS 変換の単位**: 効果を持つ関数のみ CPS 化し、純粋関数は通常呼び出しで残す（呼び出し境界でラッパを生成）。
- **Capability 連携**: `perform` 対象効果が `Stage` を持つ場合は `CapabilityRegistry` と照合する。Stage 不一致は `effects.contract.stage_mismatch` に接続する。

### 5.3.2 最小ランタイム API（ドラフト）
```c
// compiler/c/include/reml/runtime/effects.h (実装済み)
typedef struct reml_continuation reml_continuation;
typedef struct reml_effect_frame reml_effect_frame;
typedef struct reml_effect_result reml_effect_result;

typedef const char* reml_effect_tag;
typedef void* reml_effect_payload;

typedef enum {
  REML_EFFECT_RESULT_RETURN,
  REML_EFFECT_RESULT_PERFORM,
  REML_EFFECT_RESULT_PANIC
} reml_effect_result_kind;

typedef enum {
  REML_EFFECT_STATUS_OK,
  REML_EFFECT_STATUS_UNHANDLED,
  REML_EFFECT_STATUS_RESUME_TWICE,
  REML_EFFECT_STATUS_RESUME_OUT_OF_SCOPE
} reml_effect_status;

struct reml_effect_result {
  reml_effect_result_kind kind;
  reml_effect_status status;
  reml_effect_tag tag;
  reml_effect_payload payload;
  reml_continuation* cont;
};

typedef reml_effect_result (*reml_effect_fn)(void* env, reml_continuation* k);
typedef reml_effect_result (*reml_effect_handler_fn)(
  reml_effect_tag tag,
  reml_effect_payload payload,
  reml_continuation* k,
  void* handler_env
);

reml_effect_frame* reml_effect_push_handler(
  reml_effect_handler_fn handler,
  void* handler_env,
  reml_effect_frame* parent
);
reml_effect_frame* reml_effect_pop_handler(reml_effect_frame* frame);

reml_effect_result reml_effect_perform(
  reml_effect_tag tag,
  reml_effect_payload payload,
  reml_continuation* k
);
reml_effect_result reml_effect_resume(
  reml_continuation* k,
  reml_effect_payload value
);
reml_effect_result reml_effect_trampoline(
  reml_effect_fn entry,
  void* env
);
```

### 5.3.3 CPS 変換とステートマシン生成
- **MIR ノード**: `MIR_EFFECT_PERFORM`, `MIR_EFFECT_HANDLE`, `MIR_EFFECT_RESUME` を追加し、ハンドラ境界のスコープを明示化する。
- **状態表現**: CPS 関数は `{ pc, locals, handler_frame }` を持つ状態構造体とし、`pc` を `switch` で分岐。
- **継続 (`continuation`)**: `pc` と `locals` のスナップショットを保持し、`resume` で再開。再入防止のため `consumed` フラグを必須化。
- **境界の最小化**: `perform` が無い関数は CPS 化しない。CPS 関数の呼び出しは `trampoline` から開始する。

### 5.3.4 診断と安全規約
- **未捕捉効果**: `effects.unhandled`（未処理の `perform`）を追加。
- **再入禁止**: `effects.resume.already_used`（2 回目の `resume`）、`effects.resume.out_of_scope`（ハンドラ終了後の `resume`）。
- **Stage 不一致**: `effects.contract.stage_mismatch` へ接続し、`effect.stage.required/actual` をメタデータで保持。

### 5.3.5 作業ステップ（詳細）
- [x] `compiler/c/include/reml/runtime/effects.h` と `compiler/c/src/runtime/effects.c` を追加し、最小 API を定義する。
- [x] `compiler/c/include/reml/mir/mir.h` に効果用ノードを追加し、CPS 変換パスを導入する（最小の判定/フラグ更新）。
- [x] `compiler/c/src/codegen/codegen.c` に CPS 生成と `trampoline` 呼び出しの統合を追加する（最小経路）。
- [x] `compiler/c/include/reml/sema/diagnostic.h` に効果関連診断 ID を追加する。
- [ ] `Core.Effects` の最小実装として `State`/`Exception` を用意し、`perform`/`handle` を接続する。
- [x] `tests/unit` に one-shot/未処理効果/例外復帰のテストを追加する（one-shot/未処理まで）。

### 5.3.6 進捗メモ（2026-01-03）
- CPS 変換を採用し、スタックコピー方式を使わない方針を確定。
- `perform`/`resume` の one-shot 保証と `Capability Registry` 連携を必須要件として整理。
- `effects` ランタイムの最小 API と単体テスト（one-shot/未処理）を追加。
- CPS/trampoline の最小統合と、効果系診断 ID の追加を完了。

## 5.4 文字列と Unicode
- **ライブラリ**: `utf8proc` + `libgrapheme`。
- **仕様参照**: `docs/spec/1-4-test-unicode-model.md`、`docs/spec/3-3-core-text-unicode.md`。
- **タスク**:
  1.  `String` を `struct { char* ptr; size_t len; }` (UTF-8) として実装。
  2.  `Core.Text` 関数の実装 (長さ, スライス, 検証) と境界条件の整理。
  3.  正しい「文字」カウントのための書記素クラスタ (Grapheme cluster) イテレーション。
  4.  正規化 (NFC) と無効 UTF-8 の扱いを仕様に合わせて固定。
  5.  テスト: 絵文字/結合文字/幅計算/無効列の診断。

## 5.5 ADT とレコード型
- **仕様参照**: `docs/spec/1-2-types-Inference.md`、`docs/spec/1-5-formal-grammar-bnf.md`。
- **タスク**:
  1.  AST に ADT/コンストラクタ/レコード型とリテラルを追加。
  2.  パーサーで `type` 宣言、コンストラクタ呼び出し、レコードリテラル/更新を解析。
  3.  型チェック: 型引数、フィールド集合の一致、コンストラクタの引数型検査。
  4.  レイアウト: レコードのフィールド順序を仕様（正規化順）に固定。
  5.  Codegen: ADT タグ/ペイロードの表現とアクセスを実装。
  6.  パターンマッチングと連携する診断（不足/余剰フィールド、未知コンストラクタ）。
  7.  テスト: `Option`/`Result`、レコードの構築・参照・更新。

## 5.6 参照型 (`&T`, `&mut T`)
- **仕様参照**: `docs/spec/1-2-types-Inference.md`、`docs/spec/1-3-effects-safety.md`。
- **タスク**:
  1.  AST と型表現に参照型を追加し、`&`/`&mut` の構文を解析。
  2.  型チェックで不変/可変参照の整合性と再代入の制約を検証。
  3.  `mut` 効果との整合（可変参照の導入時に効果タグを付与）。
  4.  Codegen: 参照をポインタとして表現し、読み書きの命令列を定義。
  5.  テスト: `&T` の読み出し、`&mut T` の更新、参照の別名衝突診断。

## 5.7 トレイト/型クラス（演算子解決の一般化）
- **仕様参照**: `docs/spec/1-2-types-Inference.md`。
- **タスク**:
  1.  組み込みトレイト (`Add`, `Sub`, `Eq` など) の定義を型システムに統合。
  2.  演算子をトレイト解決にマッピングし、型推論と連携させる。
  3.  MVP の範囲で対象型の `impl` を固定テーブル化。
  4.  失敗時の診断（未解決/曖昧/重複）を整備。
  5.  テスト: `Int`/`Float`/`String` の演算子解決。

## 5.8 効果行 (`! Σ`)
- **仕様参照**: `docs/spec/1-2-types-Inference.md`、`docs/spec/1-3-effects-safety.md`。
- **タスク**:
  1.  関数型に効果行を保持する型表現を追加。
  2.  型推論で効果集合の合成と制約伝搬を実装。
  3.  `@pure` / `@no_panic` 等の属性と効果行の整合チェック。
  4.  診断: 効果契約違反、効果不一致のコードを確定。
  5.  テスト: 効果注釈付き関数と伝搬の挙動。

## 5.9 型推論の拡張
- **仕様参照**: `docs/spec/1-2-types-Inference.md`。
- **タスク**:
  1.  レコード/ADT/参照型を含む単一化と推論ルールの拡張。
  2.  トレイト制約を統合し、制約解決の失敗時に明確な診断を出す。
  3.  数値リテラルの既定解決と `BigInt` への昇格ルールを統一。
  4.  効果行と値制限の統合（効果がある `let` を単相化）。
  5.  テスト: 型注釈なしの推論、曖昧性診断、レコード/ADT の推論。

## 5.10 検証と完了条件
- **テスト**: `tests/unit` と `tests/integration` に追加。
- **実行確認**: `examples/spec_core` の文字列/パターン/効果を含む例の実行。
- **診断**: JSON 診断（位置情報・修正案）の出力が整合。

## チェックリスト
- [ ] `BigInt` 演算が動作する。
- [ ] Enum と Integer に対してパターンマッチングがコンパイルされる。
- [x] Integer/Bool のリテラル + ワイルドカード/識別子パターンの match がコンパイルされる。
- [x] Integer/Bool のリテラルパターンが `switch` へ降下する。
- [x] Range/Enum の最小ケースが `switch` へ降下する。
- [x] ガード付き `switch` で再評価パスが生成される。
- [ ] 文字列リテラルと基本的な文字列操作が Unicode で正しく動作する。
- [ ] 基本的な Effect Handler が動作する (少なくとも State/Exception に対して)。
- [ ] ADT/レコード型がパース・型検査・コード生成まで通る。
- [ ] 参照型 (`&T`, `&mut T`) の型規則とコード生成が動作する。
- [ ] 組み込みトレイト解決が演算子に適用される。
- [ ] 効果行 (`! Σ`) が型推論と診断に反映される。
- [ ] 型推論がレコード/ADT/参照型/BigIntを含む式で成立する。
- [ ] 主要ケースの診断 ID とエラーメッセージが整備される。
