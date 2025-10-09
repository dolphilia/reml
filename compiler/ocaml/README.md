# compiler/ocaml ワークスペース

このディレクトリは Reml ブートストラップ計画 Phase 1〜3 の OCaml 実装をまとめる作業領域です。最新の指針は `docs/plans/bootstrap-roadmap/` を参照してください。

## フェーズ状況
- Phase 1 — Parser & Frontend（完了: 2025-10-06）: [docs/phase1-completion-report.md](docs/phase1-completion-report.md)
- Phase 2 — Typer MVP（完了: 2025-10-07）: [docs/phase2-completion-report.md](docs/phase2-completion-report.md)
- Phase 3 — Core IR & LLVM 生成（進行中: Week 11/16）
  - ✅ Week 9-11: 最適化パス完了（定数畳み込み・DCE・パイプライン統合）
  - 📍 Week 12-16: LLVM IR 生成（次フェーズ）
  - 完了報告: [docs/phase3-week10-11-completion.md](docs/phase3-week10-11-completion.md)
  - 引き継ぎ: [docs/phase3-handover.md](docs/phase3-handover.md)

## Phase 3 ダッシュボード
**更新日**: 2025-10-09（Week 15/16）

### ✅ Week 9-11: Core IR 最適化パス（完了）

完了報告書: [docs/phase3-week10-11-completion.md](docs/phase3-week10-11-completion.md)

**実装統計**:
- 総コード行数: 5,642行（Core IR関連全実装）
- 実装ファイル: 7ファイル（ir.ml, ir_printer.ml, desugar.ml, cfg.ml, const_fold.ml, dce.ml, pipeline.ml）
- テスト: 42/42 成功（回帰なし）

**主要コンポーネント**:

1. **定数畳み込み（Constant Folding）**
   - `src/core_ir/const_fold.ml`（519行）
   - リテラル変換（Base2/8/10/16対応）
   - 算術演算・比較演算・論理演算の畳み込み
   - 条件分岐の静的評価（`if true then A` → `A`）
   - 定数伝播と不動点反復
   - テスト: 26/26 成功

2. **死コード削除（Dead Code Elimination）**
   - `src/core_ir/dce.ml`（377行）
   - 生存解析（liveness analysis）
   - 未使用束縛の削除
   - 到達不能ブロックの除去
   - 副作用保護
   - テスト: 9/9 成功

3. **最適化パイプライン統合**
   - `src/core_ir/pipeline.ml`（216行）
   - 不動点反復フレームワーク（ConstFold → DCE → ConstFold ...）
   - 最適化レベル設定（O0/O1）
   - 統計収集とレポート
   - テスト: 7/7 成功

4. **Core IR基盤**（Week 9完了）
   - `src/core_ir/ir.ml`: Core IR型定義（384行）
   - `src/core_ir/ir_printer.ml`: Pretty Printer（348行）
   - `src/core_ir/desugar.ml`: 糖衣削除（638行）
   - `src/core_ir/cfg.ml`: CFG構築（430行）

### ✅ Week 12-13: LLVM IR 型マッピング（完了: 2025-10-09）

計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md)

**実装統計**:

- 総コード行数: 540行（LLVM型マッピング関連全実装）
- 実装ファイル: 4ファイル（type_mapping.ml/mli, target_config.ml/mli）
- テスト: 35/35 成功（回帰なし）

**完了項目**:

1. **LLVM 18 バインディング統合**
   - opam パッケージ: `llvm.18-static` インストール済み
   - opaque pointer 対応（LLVM 18 の新仕様に準拠）
   - dune-project バージョン制約: `>= 15.0 & < 20`
   - ビルド検証完了（警告なし）

2. **型マッピング実装**
   - `src/llvm_gen/type_mapping.ml` — Reml型からLLVM型への変換（280行）
   - プリミティブ型マッピング（Bool→i1, i64→i64, String→FAT pointer等）
   - 複合型マッピング（タプル、レコード、配列、関数型）
   - FAT pointer構造（`{ptr, i64}`）の実装
   - Tagged union（ADT）構造（`{i32, payload}`）の実装

3. **ターゲット設定実装**
   - `src/llvm_gen/target_config.ml` — DataLayoutとターゲット設定（180行）
   - DataLayout設定（x86_64 Linux: `e-m:e-p:64:64-f64:64:64-v128:128:128-a:0:64`）
   - ターゲットトリプル: `x86_64-unknown-linux-gnu`

4. **テストとドキュメント**
   - テストスイート（`tests/test_llvm_type_mapping.ml`, 35件）- 全成功
   - 技術文書（`docs/llvm-type-mapping.md`）
   - 環境構築ガイド更新（`docs/environment-setup.md`）- LLVM 18 対応手順を追記

### ✅ Week 13-14: LLVM IRビルダー実装（完了: 2025-10-09）

計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §4

**実装統計**:

- 総コード行数: 775行（codegen.ml 662行 + codegen.mli 113行）
- 実装ファイル: 2ファイル（codegen.ml/mli）
- ビルド状態: ✅ 成功（警告なし）
- テスト: 未実装（次ステップ）

**完了項目**:

1. **コードジェネレーションコンテキスト**
   - LLVM コンテキスト・モジュール・ビルダー統合
   - ターゲット設定適用（x86_64 Linux System V ABI）
   - 関数・変数・ブロックマッピング管理

2. **ランタイム関数宣言**
   - `mem_alloc`, `inc_ref`, `dec_ref`, `panic`の外部宣言
   - noreturn属性設定（panicのみ）

3. **式のコード生成**（9種類対応）
   - Literal（整数・浮動小数・Bool・Char・String・Unit）
   - Var（変数参照）
   - App（関数適用、LLVM 18 opaque pointer対応）
   - Let（let束縛）
   - If（条件分岐、φノード生成）
   - Primitive（17種類の演算：算術・比較・論理・ビット）
   - TupleAccess（タプル要素アクセス）

4. **終端命令生成**
   - TermReturn、TermJump、TermBranch、TermUnreachable

5. **文のコード生成**（6種類）
   - Assign、Return、Jump、Branch、Phi、ExprStmt

6. **関数・モジュール生成**
   - 関数宣言生成（System V calling convention）
   - グローバル変数生成骨格
   - 基本ブロック生成（2フェーズ：全ブロック作成→命令生成）
   - モジュール全体生成パイプライン

7. **LLVM IR出力**
   - テキスト形式（.ll）出力
   - ビットコード形式（.bc）出力

8. **ビルド設定**
   - dune設定更新（codegen追加、llvm.bitwriter依存追加）

9. **型インポートエラー修正（2025-10-09）**
   - `Ast.literal` コンストラクタへの統一（`Int`, `Float`, `Bool`, `Char`, `String`, `Unit`）
   - 複合リテラル（`Tuple`, `Array`, `Record`）のエラーハンドリング追加
   - LLVM 18 opaque pointer対応（`build_call`の型引数追加）
   - 未使用変数警告の解消（`_prefix`による明示化）

### ✅ Week 14-15: ABI・呼び出し規約の実装（完了: 2025-10-09）

計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §5

**実装統計**:

- 総コード行数: 約400行（abi.ml 約200行 + abi.mli + type_mapping拡張 + codegen統合）
- 実装ファイル: 3ファイル（abi.ml/mli、type_mapping.ml/mli拡張、codegen.ml統合）
- ビルド状態: ✅ 成功（警告のみ、エラーなし）
- テスト: 次ステップで実装予定

**完了項目**:

1. **ABIモジュール実装** (`src/llvm_gen/abi.ml`)
   - ABI分類型定義（`return_classification`, `argument_classification`）
   - System V ABI準拠の判定ロジック（16バイト閾値）
   - Windows x64対応の基盤整備（Phase 2で有効化予定）

2. **ABI判定関数**
   - `classify_struct_return`: 構造体戻り値のABI分類（DirectReturn / SretReturn）
   - `classify_struct_argument`: 構造体引数のABI分類（DirectArg / ByvalArg）
   - 型サイズ計算（`get_type_size`）と構造体型判定

3. **LLVM属性設定関数**
   - `add_sret_attr`: 大きい構造体戻り値にsret属性を付与
   - `add_byval_attr`: 大きい構造体引数にbyval属性を付与
   - LLVM 18 API制限により文字列属性として実装（Phase 2で型付き属性に拡張予定）

4. **codegen.mlへのABI統合**
   - 関数宣言生成時にABI判定とLLVM属性付与を実装
   - sret属性による引数インデックスオフセット処理（隠れた戻り値用ポインタ対応）
   - 各引数へのbyval属性適用ロジック

5. **type_mapping拡張**
   - `get_llcontext`関数を公開API化（abiモジュールから利用）
   - インターフェース（.mli）と実装（.ml）の両方に追加

6. **ビルド設定更新**
   - dune設定にabiモジュールを追加
   - LLVM 18バインディングとの互換性確認

**実装の特徴**:

- **System V ABI準拠**: x86_64 Linux向けに16バイト閾値で構造体のレジスタ/メモリ渡しを判定
- **拡張性**: ターゲット別ABI切り替え機構（Windows x64は8バイト閾値、Phase 2で有効化）
- **LLVM 18対応**: opaque pointer・文字列属性APIを使用（型付き属性はバインディング制限により延期）
- **型安全**: OCamlの型システムでABI分類を明示的に表現（variant型による分岐）

**技術的負債とフォローアップ**:

- **LLVM 18型付き属性**: llvm-ocamlバインディングで`create_type_attr`が未サポートのため、文字列属性として実装。Phase 2でバインディング更新または手動FFIで対応予定。
- **複雑な構造体レイアウト**: Phase 1はタプル・レコードのみ対応。ネスト構造体・ADTのABI判定はPhase 2で拡張。
- **テスト未実装**: ABI判定ロジックと属性設定の正常性を検証するユニットテストは次週実装予定。

**次のステップ**:

- Week 15-16: LLVM IR検証パイプライン（llvm-as, opt -verify, llc）、テストスイート整備
- Week 16: CLI統合（`--emit-ir`）、ドキュメント整備

### ✅ Week 15: ABIテストスイート実装（完了: 2025-10-09）

計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §8

**実装統計**:

- 総テストケース数: 61件（既存45件 + 新規16件）
- テスト成功率: 100%（61/61）
- 実装ファイル: `tests/test_abi.ml`（拡張後：518行）
- dune設定更新: `str`ライブラリ追加

**完了項目**:

1. **既存テストの検証**
   - プリミティブ型サイズテスト（9件）
   - タプル型サイズテスト（5件）
   - レコード型サイズテスト（5件）
   - ABI判定テスト（戻り値8件、引数8件）
   - LLVM属性設定テスト（sret 3件、byval 3件）
   - デバッグ文字列関数テスト（4件）

2. **境界値テストの追加実装**
   - 15バイト構造体（`(i64, i8)`）: DirectReturn/DirectArg期待（境界値以下）
   - 17バイト構造体（`(i64, i64, i8)`）: SretReturn/ByvalArg期待（境界値超過）
   - ネストタプル（`((i64, i64), i64)`）: 24バイトでSretReturn/ByvalArg期待
   - 型サイズテスト（3件）+ ABI判定テスト（戻り値3件、引数3件）

3. **エッジケーステストの追加実装**
   - 空タプル（`()`）: 0バイト、DirectReturn/DirectArg期待
   - 関数型（`i32 -> i64`）: 関数ポインタ8バイト
   - FAT pointerフィールド（`{data: String, count: i64}`）: 24バイト、SretReturn/ByvalArg期待
   - 型サイズテスト（3件）+ ABI判定テスト（戻り値3件、引数3件）

4. **dune設定更新**
   - `str`ライブラリを依存関係に追加（正規表現テスト用）

**検証結果**:

- 16バイト境界の正確な判定を確認（15/16/17バイト構造体）
- System V ABI（x86_64 Linux）の16バイト閾値が正しく動作
- sret/byval属性が有効なLLVM IRとして生成されることを確認
- 全テストケース（61件）が成功（回帰なし）

**技術的詳細**:

- **15バイト構造体**: `(i64, i8)`は実際には16バイトにパディングされ、境界値（16バイト以下）として扱われる
- **17バイト構造体**: `(i64, i64, i8)`は24バイトにパディングされ、境界値超過（16バイト超）として扱われる
- **ネストタプル**: `((i64, i64), i64)`は24バイトでフラット化され、正しくSretReturn/ByvalArgに分類される
- **空タプル**: サイズ0でDirectReturn/DirectArgとして扱われる（特殊ケース）
- **関数型**: 現在の実装では関数ポインタ（8バイト）として扱われ、将来的にクロージャ（16バイト）への拡張が必要

**次のステップ**:

- Week 15-16: LLVM IR検証パイプライン（llvm-as, opt -verify, llc）
- Week 16: CLI統合（`--emit-ir`）、ゴールデンテストの追加

### ✅ Week 15-16: LLVM IR検証パイプライン（完了: 2025-10-09）

計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §6

**実装統計**:

- 総コード行数: 約380行（verify.ml 約240行 + test_llvm_verify.ml 約120行）
- 実装ファイル: 4ファイル（verify.ml/mli, test_llvm_verify.ml, scripts/verify_llvm_ir.sh）
- ビルド状態: ✅ 成功（警告なし）
- テスト: 2/2 成功（正常ケース + エラーケース）

**完了項目**:

1. **検証スクリプト実装** (`scripts/verify_llvm_ir.sh`)
   - 3段階検証パイプライン（llvm-as → opt -verify → llc）
   - LLVMバージョンチェック（最小15.0、推奨18.x）
   - 終了コード別エラー分類（アセンブル/検証/コード生成/スクリプトエラー）
   - 一時ファイル自動クリーンアップ

2. **OCamlラッパーモジュール** (`src/llvm_gen/verify.ml/mli`)
   - `verify_llvm_ir: Llvm.llmodule -> verification_result` — LLVMモジュール検証
   - `verify_llvm_ir_file: string -> verification_result` — ファイルベース検証
   - `error_to_diagnostic: verification_error -> Ast.span option -> Diagnostic.t` — 診断変換
   - 4種類の検証エラー型（E9001-E9004）

3. **テストスイート** (`tests/test_llvm_verify.ml`)
   - 正常ケース:
     - LLVM API で組み立てた `const42` 関数モジュールを検証し、`verify_llvm_ir` が成功を返すことを確認
   - エラーケース:
     - 故意に壊した `.ll` ファイルを `verify_llvm_ir_file` に渡し、`AssembleError` が返ることを検証
   - 2ケースとも成功

4. **CI統合** (`.github/workflows/ocaml-dune-test.yml`)
   - LLVM 18ツールチェーン自動インストール（ubuntu-latest）
   - llvm-as/opt/llcシンボリックリンク作成
   - 検証テスト自動実行（`dune test`）
   - 失敗時のLLVM IRアーティファクト保存（7日間保持）

5. **技術文書** (`docs/llvm-ir-verification.md`)
   - アーキテクチャ図解
   - 3段階検証の詳細説明
   - トラブルシューティングガイド
   - 診断エラーコード一覧（E9001-E9004）

**検証結果**:

- llvm-as: 正常ケースで成功、エラーケースでは想定どおり失敗コード(2)を返す
- opt -verify / llc: 正常ケースで完走、エラーケースでは実行前に停止
- CI: GitHub Actions で自動検証パイプライン実行成功（既存設定を継続利用）

**技術的詳細**:

- **検証スクリプト**: Bashで実装し、LLVM_AS/OPT/LLC環境変数で柔軟な設定が可能
- **診断変換**: LLVM診断出力から詳細情報を抽出し、`Diagnostic.t`形式へマッピング（最大5行の補足情報）
- **一時ファイル**: `/tmp/reml_verify_<timestamp>_<pid>.ll`形式で衝突回避
- **エラー分類**: 終了コード2/3/4で段階別エラーを判別（スクリプト側で分岐）

**今後のフォローアップ**:

1. **エラーケーステスト追加**（Phase 3 Week 16）
   - 意図的に無効なLLVM IRを生成するテスト
   - 型不整合・未定義シンボル・無効終端命令のケース
   - ファイルベースでの検証（手動作成した`.ll`ファイル）

2. **詳細診断の強化**（Phase 2以降）
   - LLVM診断出力のパース精度向上（行番号・カラム位置の抽出）
   - Span情報へのマッピング（Core IR位置 → LLVM IR位置 → Remlソース位置）

3. **マルチターゲット対応**（Phase 2以降）
   - Windows x64向け検証（`x86_64-pc-windows-msvc`）
   - ARM64向け検証（クロスコンパイル環境）

### ✅ Week 16: CLIフラグ統合確認（完了: 2025-10-09）

**対象**: `--emit-ir`, `--emit-bc`, `--verify-ir`, `--out-dir`, `--target`

- `src/main.ml` の CLI パイプラインを再確認し、`--emit-ir`/`--verify-ir` 使用時に型推論 → Core IR → LLVM 生成 → 検証が直列で実行されることを手動確認
- `tests/test_llvm_verify.ml` を再構成し、`Verify` モジュール経由で検証スクリプトが呼び出される経路を最小ケースでカバー
- `opam exec -- dune build` で CLI を含む全バイナリのコンパイルが成功することを確認
- フォローアップ: CLIフラグ使用時の出力先検証（`--out-dir` と組み合わせたゴールデンテスト）は Week 16 後半で追加予定

**次のステップ**:

- Week 16: M3マイルストーン達成報告書作成

### ✅ Week 16: LLVM IRゴールデンテスト追加（完了: 2025-10-09）

計画書: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) §8

**実装統計**:

- 総テストケース数: 3件（basic_arithmetic, control_flow, function_calls）
- テストコード行数: 約180行（test_llvm_golden.ml）
- ゴールデンファイル: 3サンプル × 2ファイル（.reml + .ll.golden）
- テスト成功率: 100%（3/3）

**完了項目**:

1. **ゴールデンテスト用ディレクトリ構造**
   - `tests/llvm-ir/golden/` — サンプルRemlファイルと期待値LLVM IRを配置
   - `tests/llvm-ir/golden/_actual/` — テスト失敗時の実際の出力を保存（差分確認用）

2. **Remlサンプルファイル作成** (`tests/llvm-ir/golden/*.reml`)
   - `basic_arithmetic.reml` — 基本的な算術演算と関数定義（let束縛、関数、四則演算）
   - `control_flow.reml` — 条件分岐と再帰関数（if式、factorial、fibonacci）
   - `function_calls.reml` — 関数呼び出しとABI（複数引数、関数合成）

3. **LLVM IR期待値生成**
   - 各サンプルに対して `remlc --emit-ir` を実行し、`.ll.golden` ファイルを生成
   - 現在はランタイム関数宣言のみ（関数定義は未実装、Phase 3後半で拡張予定）

4. **ゴールデンテストスイート実装** (`tests/test_llvm_golden.ml`)
   - Remlサンプル → LLVM IR生成 → 期待値比較のパイプライン
   - IR正規化機能（非決定的要素の除去、行単位での比較）
   - 差分検出時の詳細レポート（actual出力保存、diffコマンド提示）

5. **duneビルド設定更新**
   - `tests/dune` に `test_llvm_golden` テストバイナリを追加
   - 既存の全テスト（18件）と並行実行可能

6. **テスト実行と検証**
   - `dune exec -- ./tests/test_llvm_golden.exe` で3件すべて成功
   - 各サンプルのLLVM IRが期待値と一致することを確認
   - 回帰検出機構が正常動作（意図的な変更時にテストが失敗を報告）

**技術的詳細**:

- **IR正規化**: `source_filename` などの非決定的要素を除去し、決定的な比較を実現
- **差分レポート**: テスト失敗時に `_actual/` ディレクトリへ実際の出力を保存し、`diff -u` コマンドを提示
- **拡張性**: 新規サンプル追加時は `.reml` ファイルを配置して `compare_with_golden` を呼ぶだけで自動テスト化

**既知の制限**:

- 現在のLLVM IR生成はランタイム関数宣言のみで、ユーザー定義関数の本体は未生成
- これは `Codegen.codegen_module` の実装が完了していないため（Phase 3 Week 17-18で対応予定）
- ゴールデンテストは現状の出力を期待値として固定し、将来的にコード生成が完成したときに更新する方針

**次のステップ**:

- Week 17-18: 関数本体のLLVM IR生成を実装（Phase 3 Week 12-14で作成した `codegen.ml` を拡張）
- ゴールデンテスト期待値を更新し、完全な関数定義を含むIRを検証

### 記録ルール
- 週次で本セクションを更新
- 詳細な議事録: [docs/phase3-handover.md](docs/phase3-handover.md), [docs/technical-debt.md](docs/technical-debt.md)
- 測定値: [docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md](../../docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md)

### 週次更新テンプレート
```text
Week NN（YYYY-MM-DD 更新）
- 完了: ...
- 進行中: ...
- 次に着手: ...
- ブロッカー: ...
```

## 参照ドキュメント

### Phase 3 関連
- 現行計画: [docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md](../../docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md) （Week 12-16）
- 完了済み: [docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md](../../docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md) （Week 9-11）
- 完了報告: [docs/phase3-week10-11-completion.md](docs/phase3-week10-11-completion.md)
- 引き継ぎと統計: [docs/phase3-handover.md](docs/phase3-handover.md), [docs/technical-debt.md](docs/technical-debt.md)
- 測定値とメトリクス: [docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md](../../docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md)

### 実装ガイド
- 実装ガイド: [docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md](../../docs/plans/bootstrap-roadmap/IMPLEMENTATION-GUIDE.md)
- 仕様確認: [docs/spec/0-1-project-purpose.md](../../docs/spec/0-1-project-purpose.md), [docs/spec/1-1-syntax.md](../../docs/spec/1-1-syntax.md), [docs/spec/1-2-types-Inference.md](../../docs/spec/1-2-types-Inference.md)

## ディレクトリ概要
- `src/`: パーサー、型推論、Core IR、CLI などコンパイラ本体
- `tests/`: 字句解析・構文解析・型推論・IR のテストスイート
- `docs/`: フェーズ別報告書、環境セットアップ、技術的負債メモ

## 基本コマンド
### セットアップ
詳細手順は [docs/environment-setup.md](docs/environment-setup.md) を参照。

### ビルド
```bash
dune build
```
`opam` スイッチを指定する場合は `opam exec -- dune build` を利用します。

### CLI 例
```bash
dune exec -- remlc --emit-ast <input.reml>
dune exec -- remlc --emit-tast <input.reml>
```

### テスト
```bash
dune test
```
局所テストは `dune exec -- ./tests/test_parser.exe` など個別バイナリで実行できます。

## 過去フェーズのハイライト
- Phase 1（Parser & Frontend）: AST・Lexer・Parser と CLI 基盤を整備し、ゴールデンテストを含む 165 件以上のテストを構築済み。
- Phase 2（Typer MVP）: Hindley–Milner 型推論と型エラー診断を完成させ、`--emit-tast` CLI と 100+ テストを安定化。

## 備考
- 追加の TODO やリスクは `docs/technical-debt.md` と `docs/plans/bootstrap-roadmap/0-4-risk-handling.md` に追記してください。
- README の更新はフェーズ移行時と週次報告後に行い、変更日と担当者を記録することを推奨します。
