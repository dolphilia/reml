# 技術的負債リスト

**最終更新**: 2025-10-18
**Phase**: Phase 2-3 着手前（効果システム統合完了時点）

このドキュメントは、Phase 1-3 で発見された既知の問題と技術的負債を記録し、Phase 2 以降での対応を計画するものです。

## 関連ドキュメント

- **残タスク詳細**: [phase3-remaining-tasks.md](./phase3-remaining-tasks.md) - Phase 3時点の残タスクを優先度別に分類
- **引き継ぎ情報**: [phase3-handover.md](./phase3-handover.md) - Phase 2→3の成果物と前提条件

## 分類

- 🔴 **Critical**: 即座に対応が必要
- 🟠 **High**: Phase 2 で対応すべき
- 🟡 **Medium**: Phase 2-3 で対応
- 🟢 **Low**: Phase 3 以降で対応可

---

## 🟠 High Priority（Phase 2 で対応）

### 1. レコードパターンの複数アーム制限

**分類**: パーサの制限
**優先度**: 🟠 High
**発見日**: 2025-10-06

#### 問題の詳細

レコードパターンで以下の形式を複数アームで使用すると、パーサが構文エラーを報告する：

```reml
// 失敗するケース
let _ = match record with
| { x: Some(value), y } -> value + y  // 1st arm: OK
| { x: None, y } -> y                  // 2nd arm: パースエラー
```

**根本原因（2025-10-06 更新）**: `parser.mly` の `record_pattern_entry` で先頭フィールドを解析する際に、`pattern -> ident` と `primary_expr -> ident` の縮約が Menhir 上で衝突しており（`parser.conflicts` state 238/239）、裸の識別子パターンを式として確定させてしまう。結果として直後の `,` や `..` を受理できず、構文エラーを報告する。

**詳細調査結果（2025-10-06）**:
- 先頭フィールドが `field: None` や `field: Some` のような裸の識別子パターン（引数なしコンストラクタ/変数）で、直後に短縮形フィールドや `..` rest が続くと単一アームでも失敗する。
- 同じ構造でも先頭フィールドが `field: Some(value)` のように括弧付きコンストラクタであれば成功する。
- 先頭 bare コンストラクタの前に短縮形フィールドを置く、または後続フィールドを `field: pattern` 形式にすると成功する。
- エラーメッセージは常に `構文エラー: 入力を解釈できません` で、診断位置は後続フィールド先頭（例: `3:16`、`3:14`）に固定される。
- 既存のレコードパターン網羅テストでは未捕捉だったため、`compiler/ocaml/tests/test_pattern_matching.ml:333` の `test_record_pattern_limitations` を追加し、成功/失敗の境界条件を固定化した。
- `record_pattern_entry` に先頭フィールド専用の非終端を導入して `pattern -> ident` を分離する案を検証したが、Menhir の state 238/239 の reduce/reduce 衝突は解消されず、依然として `tests/tmp_record_issue.reml` が失敗することを確認した（コード変更は差分影響が大きいためロールバック済み）。
- 既存処理系の調査では、OCaml 本体がレコードパターンを専用規則 `record_pat_field` で解析し、`ラベル -> (型注釈)? -> (= パターン)?` の順で必ず `ラベル` を消費することで衝突を回避している（`/Users/dolphilia/.opam/5.2.1/.opam-switch/sources/ocaml-base-compiler.5.2.1/parsing/parser.mly:3003`）。パターン略記（pun）の場合は `= pattern` を省略しても構文木構築時に `pat_of_label` へ差し替えるため、Menhir 側で裸識別子をパターンとして扱う場面がなくなる。
- Rust (`rustc` パーサ) など LR 系実装では、先頭トークンの分類を lexer 段階で細分化し（例: `IDENT` と `FIELD_IDENT` を分ける）、さらに「パターン文脈」情報を再帰下降パーサに持たせて `record_pat_field` を式コンテキストと切り離している。Reml で同手法を採用する場合、lexer でフィールド名を区別するか、Menhir のパラメータ化非終端で「record-field context」を明示する必要がある。
- 新規アプローチとして、Lexer で先頭大文字の識別子を `UPPER_IDENT` トークンへ分類し、パターン側では `UPPER_IDENT` をゼロ引数コンストラクタとして解釈するように修正した。これにより `{ x: None, y }`・`{ x: None, .. }` など問題だったケースが成功することをパターンテスト・パーサユニットテストの両方で確認した。
- `UPPER_IDENT` 化によって `record_pattern_entry` が式文脈とトークンを共有しなくなったため、Menhir の `state 238/239` で発生していた `pattern` vs `primary_expr` の競合が解消され、既存の期待失敗テストを成功シナリオに更新済み。
- モジュール修飾付き列挙子（例: `Option.None`, `DSL.Node(tag)`）をパターンに記述するケースを追加テストし、複数セグメントの `ident` シーケンスを `PatConstructor` へ写像する規則を導入した。これにより `compiler/ocaml/tests/test_parser.ml:307` で追加したシナリオがパース可能になり、`Option.None` も `{ x: Option.None }` のように扱えることを確認した。
- ゴールデン AST テスト `tests/qualified_patterns.reml` / `tests/golden/qualified_patterns.golden` を追加し、モジュール修飾付き列挙子を含むパターンが AST に正しく反映されることをスナップショットで担保した。
- `primary_expr` は `ident`（`IDENT` と `UPPER_IDENT` を包含）を `Var` として受理する設計であり、Lexer 分割後も式文脈の挙動が変わらないことをドライバ実行で確認済み。Phase 2 で仕様書 §1-1 識別子規則へ反映する。

#### 回避策

以下のいずれかの方法で回避可能：

1. すべてのフィールドを `field: pattern` 形式に揃える：
   ```reml
   | { x: Some(value) } -> value
   | { x: None } -> 0
   ```
2. 短縮形フィールドを先頭に移動してから bare コンストラクタを記述する：
   ```reml
   | { y, x: None } -> y
   ```
3. rest パターンを使用する場合はダミーフィールドを追加して順序を変える、または rest の直前を `field: pattern` にする。

#### 対応計画

**Phase 2 Week 1-2**:
- Lexer を `IDENT` / `UPPER_IDENT` に二分し、パターン側でゼロ引数コンストラクタを正しく構築する変更を実装済み。追加で以下を確認する。
  - モジュールパス付き列挙子（例: `Option.None`）や DSL 固有の識別子が適切に分類されるかの追加テスト。
  - `UPPER_IDENT` を式文脈で `Var` として扱ってよいか（仕様レビュー）を Phase 2 タスクに追加。
- Menhir の conflict resolution を再確認し、残存する shift/reduce 警告がレコードパターンに影響しないことをレポート。
- テストスイート強化（ゴールデン AST / `--emit-ast` 出力の比較）を実施し、今回の修正で新たな回帰がないことを保証する。

**成功基準**:
- 複数アームでのレコードパターン + コンストラクタ + 短縮形が動作（`tests/test_pattern_matching.ml` 追加ケースで検証済み）
- 既存テストが全て成功（`tests/test_parser.exe` / `tests/test_pattern_matching.exe` 実行済み）
- 仕様に基づく追加シナリオ（モジュール修飾列挙子など）のテストが整備されること

### 2. AmbiguousImpl 診断の情報不足

**分類**: 診断・UX
**優先度**: 🟠 High
**ステータス**: 未対応（Phase 2-2 で対応予定）
**発見日**: 2025-10-16 / Phase 2-1 ハンドオーバー時レビュー

#### 問題の詳細
- `constraint_solver.ml` で `AmbiguousImpl` が検出されても、現在は `None` を返却して診断を生成しない。CLI 出力・JSON 出力ともに候補 impl の可視化ができず、ユーザが解決策を判断できない。
- CI で `typeclass` 系の曖昧解決が発生しても `iterator.stage.audit_pass_rate` など既存メトリクスでは検知できない。

#### 影響範囲
- 型クラス実装が本格運用に入った際、曖昧 impl が発生するとデバッグが困難。
- LSP/IDE 連携でもエラー種別が分類されず、補完ヒントが提供できない。

#### 対応計画
- `constraint_solver.ml` で `AmbiguousImpl dicts` を返却し、`Diagnostic` に `effect.stage.*` と同様の `extensions.typeclass.candidates` フィールドを追加する。
- CLI テキスト・JSON・LSP スナップショットを更新し、CI で `typeclass.ambiguous_impl_count` を監視するスクリプトを追加。
- UX 観点で「where 句追加」「impl 明示」など解決策を提示するテンプレートを整備。

#### 成功基準
- 新設する `tests/test_typeclass_ambiguous.ml` が候補 impl を含む診断を検証し、`dune runtest` が成功する。
- `0-3-audit-and-metrics.md` に曖昧 impl 件数を記録する導線が整い、CI での監視が可能になる。
- 既存診断に回帰がなく、`iterator.stage.audit_pass_rate` 指標が 1.0 を維持する。

---

## ✅ 解決済み項目（Phase 2 で完了）

### 3. Unicode XID 識別子対応

**分類**: 機能実装
**優先度**: 🟠 High
**ステータ**: ✅ 完了（2025-10-07 / Phase 2 Week 1）
**発見日**: Phase 1 開始時（計画的延期）
**解決日**: 2025-10-06

#### 実装内容

Lexer を `IDENT` / `UPPER_IDENT` に二分し、モジュール修飾付き列挙子（例: `Option.None`）をゼロ/多引数コンストラクタとして扱えるよう更新：

- `IDENT`: 小文字開始の識別子 (`[a-z_][a-zA-Z0-9_]*`)
- `UPPER_IDENT`: 大文字開始の識別子（コンストラクタ名） (`[A-Z][a-zA-Z0-9_]*`)

#### 成果

- モジュール修飾付き列挙子のサポート（`Option.None`, `Result.Ok` など）
- ゴールデンテスト追加（`tests/qualified_patterns.reml`）
- レコードパターンの複数アーム制限を解消（§1 参照）

#### 残課題

- 完全な Unicode XID（`XID_Start` + `XID_Continue*`）対応は Phase 3 以降
- 現在は ASCII のみサポート

---

### 4. AST Printer の改善

**分類**: 開発者体験
**優先度**: 🟡 Medium
**発見日**: パターンマッチ検証時

#### 問題の詳細

現在の `ast_printer.ml` はフラットな出力で、深いネスト構造が読みにくい。

**改善案**:
- インデント付き Pretty Print
- 色付き出力（オプション）
- JSON/S-expression 形式の出力

#### 対応計画

**Phase 2 Week 8**:
- Pretty Printer の実装
- `--emit-ast --format=json` オプションの追加

---

## 🟡 Medium Priority（Phase 2-3 で対応）

### 21. Windows LLVM環境とOCamlバインディング

**分類**: LLVM統合 / Windows環境
**優先度**: 🟡 Medium → ✅ 解決（方針決定）
**発見日**: 2025-10-19
**解決日**: 2025-10-19

#### 問題の詳細

Phase 2-3「FFI契約拡張」において、以下の2つの課題が明らかになった：

1. **LLVMバインディングのビルド失敗**
   - `opam install llvm` が `conf-llvm-static.19` のビルド失敗でエラー
   - 原因: LLVM静的ライブラリ（.lib）が見つからない
   - 環境: MSYS2 LLVM 16.0.4は動的ライブラリ（.dll）のみ提供

2. **LLVMバージョン不一致**
   - 要求: LLVM 18.0+（Phase 2計画書）
   - 現状: MSYS2 LLVM 16.0.4
   - 影響: Phase 2では互換性があるが、Phase 3で要検討

#### 調査結果

LLVM 18.1.8のソースビルドを試行した結果、以下の結論に至った：

**Phase 2-3での推奨**: **MSYS2 LLVM 16.0.4の継続利用**

**根拠**:

- LLVM 16と18のIR互換性は高い（Opaque Pointer移行完了）
- Remlコンパイラは基本的なLLVM機能のみ使用
- OCamlバインディングなしでFFI経由のllc/opt使用で回避可能（既存実装）
- ソースビルドのコスト: ビルド時間2-4時間、ディスク50GB、ABI混在リスク

#### 対応方針

1. **OCamlバインディング**: 使用しない
   - `compiler/ocaml/src/llvm_gen/`の外部プロセス呼び出しで対応（既存実装）
   - `opam install llvm`は試行しない

2. **LLVM 16.0.4継続使用**:
   ```bash
   export PATH="/c/msys64/mingw64/bin:$PATH"
   llc --version  # LLVM 16.0.4
   opt --version  # LLVM 16.0.4
   ```

3. **Target Triple**: `x86_64-w64-windows-gnu`で進行

4. **Phase 3以降での対応**:
   - MSYS2でLLVM 18パッケージが提供されたら即座に移行
   - LLVM 18固有機能が必要になった時点で再評価

#### 参照資料

- [windows-llvm-build-investigation.md](../../../docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md) - 詳細な調査報告
- [2-3-windows-local-environment.md](../../../docs/plans/bootstrap-roadmap/2-3-windows-local-environment.md) - Windows環境構築メモ

### 22. Windows Capability Stage 自動検証不足

**分類**: CI / Runtime Capability  
**優先度**: 🟡 Medium → ✅ 完了（2025-10-25 / Phase 2-4 Week 0）  
**発見日**: 2025-10-18

#### 対応結果（2025-10-25 更新）
- `.github/workflows/bootstrap-windows.yml` に Bash ベースの監査ジョブを追加し、`windows-latest` 上で `tooling/ci/collect-iterator-audit-metrics.py`（`--audit-source cli-ffi-bridge-windows.jsonl.golden` を指定）と `tooling/ci/sync-iterator-audit.sh` を実行。`bridge.platform` / `iterator.stage.audit_pass_rate` を CI で検証するようにした。
- `collect-iterator-audit-metrics.py` を拡張し、監査ログ（JSON/JSONL）を `--audit-source` で取り込み可能にしたことで、Windows 成功ログが欠落した場合に `ffi_bridge.audit_pass_rate` が低下し CI が失敗する。
- 生成物（`reports/iterator-stage-summary-windows.md`, `tooling/ci/iterator-audit-metrics.json`）をアーティファクト化し、レビュー時に Windows Stage override と `bridge.platform` の整合を確認できるようにした。
- **追記（2025-11-06）**: `tooling/ci/collect-iterator-audit-metrics.py --platform windows-msvc --require-success` を導入し、`bootstrap-windows.yml#audit` で Stage/FFI 監査が 1.0 未満の場合に自動で失敗する構成へ更新。生成サマリ (`reports/iterator-stage-summary-windows.md`) の `platform_summary.windows-msvc` を 1.0 で固定し、ID22 の検証ログを `docs/plans/bootstrap-roadmap/2-7-deferred-remediation.md` に反映。

#### 参照
- `.github/workflows/bootstrap-windows.yml`
- `tooling/ci/collect-iterator-audit-metrics.py`
- `tooling/ci/sync-iterator-audit.sh`

### 23. macOS FFI サンプル (`ffi_dispatch_async`) の自動検証不足

**分類**: ランタイム / テスト  
**優先度**: 🟡 Medium → ✅ 完了（2025-10-25 / Phase 2-4 Week 0）  
**発見日**: 2025-10-24

#### 対応結果（2025-10-25 更新）

- `examples/ffi/macos/ffi_dispatch_async.reml` / `examples/ffi/macos/ffi_malloc_arm64.reml` を追加し、Darwin AAPCS64 の `dispatch_async_f` と `malloc`/`free` パスを CI サンプルとして固定化。所有権情報（`bridge.return.*`）と `bridge.platform=macos-arm64` を監査ログに含めた。
- `.github/workflows/bootstrap-macos.yml` の `iterator-audit` ジョブで上記サンプルを `remlc --emit-audit` 付きで実行し、`tooling/ci/ffi-audit/macos/*.audit.jsonl` を生成。`collect-iterator-audit-metrics.py` の `--audit-source` で投入しない場合は `ffi_bridge.audit_pass_rate` が 1.0 にならず CI が失敗する。
- `tooling/ci/sync-iterator-audit.sh` に `--macos-ffi-samples` オプションを追加し、macOS 成功ログを Markdown サマリーに表示。`reports/iterator-stage-summary.md` で macOS FFI サンプルの検証結果を確認できるようにした。
- `reports/ffi-bridge-summary.md` / `reports/ffi-macos-summary.md` を更新し、macOS 固有サンプルが自動実行される旨と生成ログの保存先を明記。
- **追記（2025-11-06）**: `collect-iterator-audit-metrics.py --platform macos-arm64 --require-success` を `bootstrap-macos.yml`（`audit-matrix` / `iterator-audit` ジョブ）へ適用し、macOS FFI 監査の欠落時に CI が失敗することを確認。`reports/iterator-stage-summary-macos.md` に `platform_summary.macos-arm64` の 1.0 維持ログを追加し、ID23 をクローズ済みとして `docs/plans/bootstrap-roadmap/2-3-to-2-4-handover.md` に同期。

#### 参照
- `examples/ffi/macos/ffi_dispatch_async.reml`
- `examples/ffi/macos/ffi_malloc_arm64.reml`
- `.github/workflows/bootstrap-macos.yml`
- `tooling/ci/collect-iterator-audit-metrics.py`
- `tooling/ci/sync-iterator-audit.sh`
- `reports/ffi-bridge-summary.md`
- `reports/ffi-macos-summary.md`
- **フォローアップ（2025-02-XX 追加）**:
  - `scripts/ci-verify-llvm-link.sh` の実行結果（`ci-verify-llvm-link.md`）を macOS `audit-matrix` ジョブのアーティファクトとして保存し、`llvm.link.status = success` を完了条件に追加。
  - `docs/notes/macos-ci-llvm-link-error-report.md` に最新 run ID と検証ログ（`opam exec -- llvm-config ...`, `otool -L` など）を記録する。
  - `collect-iterator-audit-metrics.py` 拡張で `llvm.link.status` を `ffi_bridge.audit_pass_rate` のゲートに含め、リンク不整合を検出した場合は CI を失敗させる。

### 11. 統計機能の拡張

**分類**: CLI / メトリクス
**優先度**: 🟡 Medium
**ステータス**: 部分完了（基礎実装済み、Phase 2 で拡張）
**発見日**: 2025-10-10 / Phase 1-6

#### 問題の詳細

Phase 1-6 で統計情報収集の基本機能（`Cli.Stats`）は実装済みだが、以下の機能が未実装：

**現状**:
- トレースサマリーと統計カウンタの基本実装完了
- JSON 出力機能（`Cli.Stats.to_json`）実装済み

**未実装**:
- `--metrics` フラグとファイル出力（`--metrics <path>`）
- JSON/CSV スキーマの正式化（`docs/schemas/remlc-metrics.schema.json`）
- `0-3-audit-and-metrics.md` への自動書き出しスクリプト
- フェーズ別ランキング出力（時間比率順のソート）
- 10MB 入力計測プロファイルの確立

#### 影響範囲

- CI でのメトリクス記録が手動になる
- 性能回帰の自動検出ができない
- メトリクスの長期追跡が困難

#### 対応計画

**Phase 2 Week 17-20**:
- `--metrics` フラグの実装
- JSON/CSV 出力スキーマの策定
- CI 連携スクリプトの作成（`tooling/ci/record-metrics.sh` の拡張）

**Phase 2 Week 20-30**:
- フェーズ別ランキング機能の実装
- 10MB 入力ファイルでの性能測定
- 基準値の設定と記録

**成功基準**:
- `--metrics <path>` で JSON/CSV 出力が可能
- CI で自動的にメトリクスが記録される
- `0-3-audit-and-metrics.md` に時系列データが蓄積される

---

### 12. CLI 統合テストの完全な網羅

**分類**: テスト / 品質保証
**優先度**: 🟡 Medium
**ステータス**: 部分完了（診断テストのみ、Phase 2 で拡張）
**発見日**: 2025-10-10 / Phase 1-6

#### 問題の詳細

Phase 1-6 で診断出力テスト（`test_cli_diagnostics.ml`）は完了したが、以下のテストが未実装：

**実装済み**:
- 診断出力テスト（テキスト、JSON、カラーコード）
- トレース/統計テスト（基本動作）

**未実装**:
- 各オプションの網羅的動作検証（`--emit-*` 全パターン）
- エラー時の正しい終了コード確認
- スモークテストの完全実装
- 長時間実行時の安定性テスト

#### 影響範囲

- CLI オプションの回帰テストが不十分
- エラーハンドリングの検証が不完全
- CI でのカバレッジが限定的

#### 対応計画

**Phase 2 Week 17-20**:
- スモークテストの実装（`tests/test_cli_smoke.ml`）
- 終了コード検証テストの追加
- `--emit-*` オプション全パターンのテスト

**Phase 2 Week 20-30**:
- 長時間実行テストの追加（10MB 入力）
- エラーハンドリングの網羅的テスト
- CI でのテストカバレッジ測定

**成功基準**:
- 全 CLI オプションがテストでカバーされる
- エラーケースの終了コードが正しく検証される
- CI でのテストカバレッジが 90% 以上

### 13. 静的ベンチマークの IR/BC 生成不足

**分類**: ベンチマーク基盤
**優先度**: 🟡 Medium
**ステータス**: 未対応（Phase 2-2 で対応予定）
**発見日**: 2025-10-16 / Phase 2-1 → 2-2 ハンドオーバー

#### 問題の詳細
- `benchmark_typeclass.sh --static-only` が辞書渡し／モノモルフィゼーション比較用の JSON を生成するようになったが、while/for 未実装のため `benchmarks/micro_typeclass.reml` が IR・ビットコードを生成せず、静的メトリクスがすべて `0` となっている。
- 静的比較がゼロ値のままでは、効果システム統合で追加する IR の影響評価ができない。

#### 影響範囲
- Phase 2-2 の着手前チェックリストでは静的比較レポートの生成が必須だが、有効な差分が得られない。
- Phase 3 でループ実装が完了するまで辞書構造体のサイズ変化を検証できない。

#### 対応計画
- ループを使用せずに大量呼び出しを発生させるユーティリティ（例: `repeat_call(impl, n)`）を `benchmarks/micro_typeclass.reml` に追加し、`--static-only` 実行時に IR/BC を出力させる。
- 静的比較 JSON を GitHub Actions アーティファクトとして保存し、`0-3-audit-and-metrics.md` の週次ログへ転記する運用を整備。
- Phase 3 で while/for 実装が完了した際に実行ベンチと比較し、静的メトリクスが同傾向で推移することを確認する。

#### 成功基準
- `benchmark_typeclass.sh --static-only` の実行結果で `ir_lines > 0` / `bitcode_size > 0` が得られる。
- 静的比較 JSON が CI アーティファクト化され、`0-3-audit-and-metrics.md` に差分が記録される。
- 実行ベンチ再開後も静的メトリクスとの整合性が確認できる。

---

### 8. 配列リテラルの型推論

**分類**: 型推論の未実装機能
**優先度**: 🟡 Medium
**ステータス**: 未対応（Phase 2 で延期）
**発見日**: 2025-10-07 / Phase 2 Week 8

#### 問題の詳細

配列リテラル `[1, 2, 3]` の型推論は未実装：

```reml
// 現在エラーになるケース
let arr = [1, 2, 3]  // 型推論失敗
```

タプルやレコードリテラルは実装済みだが、配列リテラルのみ未対応。

#### 影響範囲

- 配列リテラルが使用できない
- 回避策: 標準ライブラリの配列構築関数を使用

#### 対応計画

**Phase 3 前半**:
- `infer_literal` 関数に配列リテラル処理を追加
- 要素型の推論と統一
- 固定長配列 `[T; N]` vs 動的配列 `[T]` の区別

**成功基準**:
- 配列リテラルの型推論成功
- 要素型の統一が正しく動作
- 型エラーケースのテスト追加

---

### 9. CFG構築時の到達不能ブロック生成

**分類**: Core IR / CFG構築アルゴリズム
**優先度**: 🟡 Medium → ✅ 解消
**ステータス**: 対応済（Phase 3 Week 10）
**発見日**: 2025-10-07 / Phase 3 Week 10

#### 問題の詳細

ネストした制御フロー構造（特にネストした if 式）を `CFG.build_cfg_from_expr` で変換すると、テストベンチが大量の到達不能ブロック警告を出していた。

```ocaml
(* if cond1 then (if cond2 then e1 else e2) else e3 *)
let inner_if = IR.make_expr (IR.If (cond2, e1, e2)) ty_i64 dummy_span in
let outer_if = IR.make_expr (IR.If (cond1, inner_if, e3)) ty_i64 dummy_span in
let blocks = CFG.build_cfg_from_expr outer_if in
```

**修正前のテスト観測値**:
- `test_unreachable_detection`: 到達不能ブロック数 3
- `test_nested_if`: `if_then_1`, `if_then_4`, `if_else_5`, `if_merge_6`, `if_else_2`, `if_merge_3` の6ブロックが警告対象

#### 根本原因

`compiler/ocaml/src/core_ir/cfg.ml` の `find_unreachable_blocks` が、探索開始時にエントリブロックを先に `Hashtbl.add` してしまい、再帰 DFS が即座に終了していた。その結果、実際には接続されている then/else/merge ブロックが訪問されず、すべて未到達扱いになっていた。

#### 対応内容（2025-10-07）

- `find_unreachable_blocks` を全面リライトし、以下を実施
  - ブロックラベル→ブロック本体のルックアップテーブルを事前構築
  - エントリブロックを訪問済み扱いする前に DFS へ渡し、`TermBranch` / `TermJump` / `TermSwitch` の後続を漏れなく再帰訪問
  - 未定義ラベルは解析対象外とし、`validate_cfg` 側のエラー検知に委譲
- 変更ファイル: `compiler/ocaml/src/core_ir/cfg.ml`

#### 検証結果

```
dune exec -- ./tests/test_cfg.exe
  → test_unreachable_detection: 到達不能ブロック 0
  → test_nested_if: 警告なしで通過
```

ネストした if を含む 118 件の既存テストを再実行し、回帰がないことを確認済み。

#### 今後のフォローアップ

1. 定数畳み込みを導入した後、静的に到達不能となるブランチを除去する拡張を検討
2. CFG 可視化（Graphviz など）を追加し、複雑な制御フローのデバッグを容易にする
3. SSA 変換パス導入時に支配関係解析と統合し、探索ロジックを再検証

---

## 🟡 Medium Priority（Phase 2-3 で対応）

### 7. 型エラー生成順序の問題

**分類**: 型推論エンジンの設計
**優先度**: 🟠 High
**ステータス**: ✅ 完了（2025-10-07 / Phase 2 Week 10）
**発見日**: 2025-10-07（Phase 2 Week 9）

#### 問題の詳細

現在の型推論エンジンは**制約ベースの双方向型推論**を実装していますが、一部のエラーケースで期待されるエラー型ではなく、汎用的な `UnificationFailure` が報告されます。これは、型推論の順序と単一化のタイミングに起因します。

**失敗するテストケース（7件）**:

1. **E7007: BranchTypeMismatch** - if式の分岐型不一致
   - 期待: `BranchTypeMismatch { then_ty: i64, else_ty: String, ... }`
   - 実際: `UnificationFailure (i64, String, ...)`
   - 原因: 253行目の `unify` が汎用的な型不一致エラーを返す

2. **E7005: NotAFunction** (2件) - 非関数型への関数適用
   - 期待: `NotAFunction (i64, ...)`
   - 実際: `UnificationFailure (i64, (i64 -> t0), ...)`
   - 原因: `infer_call` で関数型を期待する際の単一化エラー

3. **E7006: ConditionNotBool** (2件) - 条件式が非Bool型
   - 期待: `ConditionNotBool (i64, ...)`
   - 実際: `UnificationFailure (i64, Bool, ...)`
   - 原因: 241行目の `unify s1 cond_ty ty_bool` が汎用エラーを返す

4. **E7014: NotATuple** - 非タプル型へのタプルパターン
   - 期待: `NotATuple (i64, ...)`
   - 実際: `UnificationFailure (i64, (t0, t1), ...)`
   - 原因: パターン推論での型構築時の単一化エラー

#### 根本原因

現在の実装では、`Constraint.unify` が常に `UnificationFailure` を返します。しかし、**文脈に応じた専用エラー型**を生成するには、呼び出し側で型チェックの意図を認識し、適切なエラーを構築する必要があります。

例:
```ocaml
(* 現在の実装 *)
let* s2 = unify s1 cond_ty ty_bool cond.expr_span in  (* UnificationFailure を返す *)

(* 理想的な実装 *)
match unify s1 cond_ty ty_bool cond.expr_span with
| Ok s2 -> ...
| Error _ when not (is_bool cond_ty) -> Error (ConditionNotBool (cond_ty, cond.expr_span))
| Error e -> Error e
```

#### 影響範囲

- **機能面**: エラーは正しく検出されるが、診断メッセージの品質が低下
- **ユーザー体験**: 「型不一致」という汎用的なメッセージではなく、「条件式はBool型が必要です」のような具体的なメッセージが望ましい
- **テスト（修正前）**: 24件中7件が失敗（診断品質の検証ができない）
- **テスト（修正後）**: 論理演算・matchガード・パターンガード・パイプ演算子を含む30件すべて成功

#### 対応計画

**Phase 2 後半（Week 10-12）**:
1. `unify` 呼び出しの文脈を解析し、以下のパターンで専用エラーを生成：
   - `unify expected_ty actual_ty` の直後に型カテゴリをチェック
   - 関数適用コンテキスト → `NotAFunction`
   - 条件式コンテキスト → `ConditionNotBool`
   - 分岐型統一コンテキスト → `BranchTypeMismatch`
   - パターンマッチコンテキスト → `NotATuple`, `NotARecord`

2. ヘルパー関数の導入:
   ```ocaml
   val unify_as_function : substitution -> ty -> span -> (substitution * ty, type_error) result
   val unify_as_bool : substitution -> ty -> span -> (substitution, type_error) result
   val unify_branches : substitution -> ty -> ty -> span -> (substitution, type_error) result
   ```

3. エラー判定ロジックの追加:
   ```ocaml
   let is_function_type = function TArrow _ -> true | _ -> false
   let is_bool_type ty = type_equal ty ty_bool
   let is_tuple_type = function TTuple _ -> true | _ -> false
   ```

**Phase 3**:
- 型推論エンジンの全面的なリファクタリング（必要に応じて）
- より高度な型エラー回復戦略の実装

#### 回避策

現在のテストでは、以下の方針で対処：
- 汎用的な `UnificationFailure` を許容する
- 重要なのは**エラーが検出されること**であり、エラー型の精度は次優先
- `test_type_errors.ml` の該当テストは KNOWN ISSUE としてマーク

**成功基準**:
- 全7件のテストが専用エラー型を報告するように修正
- エラーメッセージが仕様書 2-5 の診断フォーマットに準拠
- 既存の成功テスト（17件）が引き続き成功

#### 診断統合での新たな発見（2025-10-07 Week 9-10）

**背景**: CLI統合と診断出力の改善タスクにて、`Type_error.to_diagnostic` の実装を完了しました。

**判明した事実**:

1. **診断変換は正常に動作**
   - 全15種類の型エラー（E7001-E7015）に対する日本語診断メッセージの生成は成功
   - `to_diagnostic_with_source` により、正確な行列番号を含む診断が生成される
   - FixIt（修正提案）の自動生成も機能している

2. **問題の本質は診断変換ではなく型推論側**
   - `to_diagnostic` は受け取った `type_error` を正しく診断に変換する
   - しかし、型推論エンジンが `UnificationFailure` を返す時点で、文脈情報が失われている
   - 診断層での補正では、失われた文脈を復元できない

3. **具体的な診断出力例**:
   ```bash
   # ConditionNotBool が期待されるケース（実際は UnificationFailure）
   /tmp/diagnostic_test.reml:2:19: エラー[E7001] (型システム): 型が一致しません
   補足: 期待される型: i64
   補足: 実際の型:     Bool
   ```
   - エラーコードが E7001（汎用的な型不一致）になっている
   - 本来は E7006（条件式がBool型でない）が正しい

4. **影響の明確化**:
   - ユーザーは型エラーを**検出できる**（機能は正常）
   - しかし、エラーの**文脈情報**（「条件式として使われている」など）が失われる
   - 診断メッセージの品質が低下し、修正方法が分かりにくくなる

**結論**:
- 診断統合タスクは完了したが、根本的な問題（型推論エンジン側）は残存
- 対応は Phase 2 後半（Week 10-12）で型推論エンジンの修正として実施する必要がある
- 診断システム側の準備は完了しており、型推論エンジンが正しいエラー型を返せば、即座に高品質な診断が出力される

**追加推奨事項**:

1. **文脈依存の unify ヘルパー関数の実装**（優先度: High）
   ```ocaml
   (* Type_inference.ml に追加 *)
   let unify_as_bool s ty span =
     match unify s ty ty_bool span with
     | Ok s' -> Ok s'
     | Error _ -> Error (ConditionNotBool (ty, span))

   let unify_as_function s ty span =
     match ty with
     | TArrow _ -> Ok s
     | _ -> Error (NotAFunction (ty, span))
   ```

2. **型推論の各コンテキストで専用ヘルパーを使用**
   - `infer_if` → `unify_as_bool` を使用
   - `infer_call` → `unify_as_function` を使用
   - 分岐の型統一 → 専用エラー `BranchTypeMismatch` を生成

3. **テスト駆動での修正**
   - 失敗している7件のテストケースを一つずつ修正
   - 各修正が他のテスト（39件）を破壊しないことを確認

**優先順位の再評価**:
- 当初 🟡 Medium Priority としていたが、診断品質への影響が大きいため 🟠 High Priority に引き上げを推奨
- Phase 2 完了前（Week 10-12）に対応することで、ユーザー体験が大幅に向上する

#### 対応結果（2025-10-07 Phase 2 Week 10）

- `compiler/ocaml/src/type_inference.ml` に文脈依存のヘルパー関数
  `unify_as_bool`・`unify_branch_types`・`unify_as_function` を追加し、
  条件式・分岐・関数適用の各パスで専用エラーを生成するよう修正。
- `infer_binary_op` の論理演算子、`if` 式、`match` アーム、
  パターンガード、`|>` パイプ演算、およびタプルパターン分岐が
  新しいヘルパーを利用するように更新され、`ConditionNotBool`
  `BranchTypeMismatch` `NotAFunction` `NotATuple` が適切に出力される。
- `compiler/ocaml/tests/test_type_errors.ml` の期待値と追加サンプルを更新し、
  `dune exec -- ./tests/test_type_errors.exe` で 30/30 件すべて成功を確認。
- 既存の回避策（汎用 `UnificationFailure` の許容）は撤廃でき、
  Phase 2 後半以降のタスクは追加最適化フェーズへ移行可能。
- 残課題: 追加で導入予定の効果システムや型クラス導入時に同様の
  ヘルパーが必要になる可能性があるため、Phase 2-3 の設計レビューで
  再評価する。

---

## 🟡 Medium Priority（Phase 2-3 で対応）（続き）

### 14. CI実行時間の最適化

**分類**: CI / インフラ
**優先度**: 🟡 Medium
**ステータス**: 未対応（Phase 2 で改善）
**発見日**: 2025-10-10 / Phase 1-7

#### 問題の詳細

Phase 1-7 で GitHub Actions の CI パイプラインを構築したが、以下の課題が残っている：

**現状**:

- 全ステージ（Lint, Build, Test, LLVM Verify, Record Metrics, Artifact）の実行時間が約 15-20 分
- キャッシュヒット時でも 10-15 分程度
- LLVM 18 のインストールに約 2-3 分
- OCaml/opam の依存関係インストールに約 3-5 分

**改善余地**:

- Docker イメージによる事前ビルド（LLVM + OCaml 環境）
- ジョブの並列化（現在は直列依存）
- キャッシュ戦略の最適化（より細かい粒度）
- 不要なステップの削減

#### 影響範囲

- 開発者の待ち時間増加
- GitHub Actions の無料枠消費
- PR レビューの遅延

#### 対応計画

**Phase 2 Week 17-20**:

- Docker イメージの作成と GitHub Container Registry への発行
- ジョブの並列化可能性の検討
- キャッシュ戦略の見直し

**Phase 2 Week 20-30**:

- セルフホストランナーの導入検討
- 実行時間の測定と最適化
- 目標: 全ステージ 10 分以内（キャッシュヒット時 5 分以内）

**成功基準**:

- CI 実行時間が 10 分以内に短縮される（キャッシュヒット時）
- Docker イメージが正しくキャッシュされる
- 開発者体験が向上する

---

### 15. メトリクス自動記録の精度向上

**分類**: CI / メトリクス
**優先度**: 🟡 Medium
**ステータス**: 部分完了（基礎実装済み、Phase 2 で改善）
**発見日**: 2025-10-10 / Phase 1-7

#### 問題の詳細

Phase 1-7 で `tooling/ci/record-metrics.sh` を実装したが、以下の機能が未実装または精度に課題がある：

**現状**:

- ビルド時間の記録は手動パラメータ渡し
- テスト件数の記録は手動パラメータ渡し
- メトリクスの記録先が手動更新（`0-3-audit-and-metrics.md`）

**未実装**:

- GitHub Actions の実行ログからの自動解析
- ビルド時間の正確な計測（各ステージ別）
- テスト件数の自動カウント
- メトリクスの JSON/CSV 形式での記録
- 時系列グラフの自動生成

#### 影響範囲

- メトリクスの精度が低い
- 手動更新の手間
- 性能推移の追跡が困難

#### 対応計画

**Phase 2 Week 17-20**:

- GitHub Actions のログ解析スクリプト作成
- ビルド時間の自動計測（`time` コマンド利用）
- テスト件数の自動カウント（`dune runtest` 出力解析）

**Phase 2 Week 20-30**:

- JSON/CSV 形式でのメトリクス記録
- `0-3-audit-and-metrics.md` への自動追記スクリプト
- 時系列グラフの生成（gnuplot または Python）

**成功基準**:

- メトリクスが完全自動で記録される
- ビルド時間とテスト件数が正確に計測される
- 時系列グラフが自動生成される

---

## 🟢 Low Priority（Phase 3 以降）

### 16. カバレッジレポートの生成

**分類**: テスト / 品質保証
**優先度**: 🟢 Low
**ステータス**: 未対応（Phase 2-3 で実装）
**発見日**: 2025-10-10 / Phase 1-7

#### 問題の詳細

Phase 1-7 で CI パイプラインを構築したが、テストカバレッジの計測と可視化は未実装：

**実装済み**:

- 単体テスト（143件）の実行
- ゴールデンテストの実行
- テスト結果の JUnit XML 出力

**未実装**:

- コードカバレッジの計測（`bisect_ppx`）
- カバレッジレポートの生成
- CI でのカバレッジレポート保存
- カバレッジの時系列追跡
- カバレッジバッジの表示

#### 影響範囲

- テストカバレッジが不明
- テスト漏れの検出が困難
- コード品質の可視化が不十分

#### 対応計画

**Phase 2 Week 20-30**:

- `bisect_ppx` の導入と設定
- カバレッジレポート生成スクリプトの作成
- CI でのカバレッジ計測の自動化

**Phase 3 前半**:

- カバレッジの時系列追跡
- カバレッジバッジの README への追加
- カバレッジ目標値の設定（例: 80% 以上）

**成功基準**:

- CI でカバレッジが自動計測される
- カバレッジレポートが HTML 形式で生成される
- カバレッジが 80% 以上を維持される

---

### 13. ベンチマークスイートの作成

**分類**: 性能測定 / 品質保証
**優先度**: 🟢 Low
**ステータス**: 未対応（Phase 2-3 で実装）
**発見日**: 2025-10-10 / Phase 1-6

#### 問題の詳細

Phase 1-6 で基本的なサンプルコード（`examples/cli/*.reml`）は整備されたが、性能測定用のベンチマークスイートは未実装：

**実装済み**:
- 小規模サンプルコード（数行〜数十行）
- トレース機能による基本的な性能測定

**未実装**:
- 性能測定用のサンプルセット（1KB〜10MB）
- 回帰テストの基準値設定
- ベンチマーク実行スクリプト
- CI での定期実行

#### 影響範囲

- 性能回帰の自動検出ができない
- 最適化の効果測定が困難
- スケーラビリティの検証が不十分

#### 対応計画

**Phase 2 Week 20-30**:
- ベンチマーク用サンプルコードの作成（1KB, 100KB, 1MB, 10MB）
- 基準値の測定と記録

**Phase 3 前半**:
- ベンチマーク実行スクリプトの作成
- CI での定期実行設定
- 性能回帰の自動検出

**成功基準**:
- 複数サイズのサンプルコードが用意される
- 基準値が `0-3-audit-and-metrics.md` に記録される
- CI で定期的にベンチマークが実行される
- 性能回帰が自動検出される

---

### 5. 性能測定の未実施

**分類**: 計測・最適化
**優先度**: 🟢 Low
**計画**: Phase 3

#### 内容

Phase 1 で以下の性能測定が未実施：

- 10MB ソースファイルの解析時間
- メモリ使用量のプロファイリング
- O(n) 特性の検証

#### 対応計画

**Phase 3**:
- ベンチマークスイートの作成
- 性能測定と最適化
- [0-3-audit-and-metrics.md](../../../docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md) への記録

---

### 6. エラー回復の強化

**分類**: 診断品質
**優先度**: 🟢 Low
**計画**: Phase 3

#### 改善案

- 期待トークン集合の提示
- より詳細な診断メッセージ
- 複数エラーの同時報告

#### 対応計画

**Phase 3**:
- エラー回復戦略の実装
- 診断メッセージの改善

---

## 除外項目（対応不要）

## 対応状況トラッキング

| ID | 項目 | 優先度 | ステータス | 担当 Phase | 備考 |
|----|------|--------|-----------|-----------|------|
| 1  | レコードパターン複数アーム | 🟠 High | ✅ 完了 | Phase 2 W1-2 | Lexer 分割 + テスト強化 |
| 2  | Unicode XID（モジュール修飾） | 🟡 Medium | ✅ 完了 | Phase 2 W1 | IDENT/UPPER_IDENT 分割 |
| 2b | Unicode XID（完全対応） | 🟢 Low | 未対応 | Phase 3-4 | uutf/uucp ライブラリ |
| 3  | AST Printer 改善 | 🟡 Medium | 未対応 | Phase 3 | Pretty Print |
| 4  | 性能測定 | 🟢 Low | 未対応 | Phase 3 | ベンチマーク |
| 5  | エラー回復強化 | 🟢 Low | 未対応 | Phase 3 | 診断改善 |
| 6  | 型エラー生成順序 | 🟠 High | ✅ 完了 | Phase 2 W10 | 文脈ヘルパー導入・`test_type_errors` 30/30 成功 |
| 7  | Handler 宣言パース | 🟠 High | ✅ 完了 | Phase 2 開始前 | `handler_entry` 導入 |
| 8  | 配列リテラル型推論 | 🟡 Medium | 未対応 | Phase 3 前半 | `infer_literal` 拡張 |
| 9  | CFG構築時の到達不能ブロック生成 | 🟡 Medium | ✅ 完了 | Phase 3 W10 | `find_unreachable_blocks` 修正完了 |
| 10 | LLVM 18型付き属性 | 🟡 Medium | ✅ 完了 | Phase 1-5 | `Llvm_attr` FFI で解消 |
| 11 | 統計機能の拡張 | 🟡 Medium | ⏸️ 部分完了 | Phase 2 W17-30 | 基礎実装済み、`--metrics` 等は未実装 |
| 12 | CLI 統合テストの完全な網羅 | 🟡 Medium | ⏸️ 部分完了 | Phase 2 W17-30 | 診断テストのみ完了 |
| 13 | ベンチマークスイートの作成 | 🟢 Low | 未対応 | Phase 2-3 | サンプルは整備済み |
| 14 | CI実行時間の最適化 | 🟡 Medium | 未対応 | Phase 2 W17-30 | Phase 1-7 で発見 |
| 15 | メトリクス自動記録の精度向上 | 🟡 Medium | ⏸️ 部分完了 | Phase 2 W17-30 | 基礎実装済み、自動解析未実装 |
| 16 | カバレッジレポート生成 | 🟢 Low | 未対応 | Phase 2-3 | Phase 1-7 で発見 |

---

## ✅ 解決済み項目

- **2025-10-06**: Handler 宣言のパースを仕様準拠に更新し、`tests/test_parser.ml` の TODO ケースを廃止（`compiler/ocaml/src/parser.mly` の `handler_body` を `handler_entry` 列挙へ置換）。

### 9. CFG構築時の到達不能ブロック生成（解決済み）

**分類**: Core IR / CFG構築アルゴリズム
**優先度**: 🟡 Medium → ✅ 完了
**ステータス**: 解決済み（Phase 3 Week 10）
**発見日**: 2025-10-07 / Phase 3 Week 10
**解決日**: 2025-10-07

#### 問題の詳細

ネストした制御フロー構造（特にネストした if 式）を `CFG.build_cfg_from_expr` で変換すると、到達不能ブロック検出が誤作動していた。

#### 根本原因

`compiler/ocaml/src/core_ir/cfg.ml` の `find_unreachable_blocks` が、探索開始時にエントリブロックを先に `Hashtbl.add` してしまい、再帰 DFS が即座に終了していた。その結果、実際には接続されている then/else/merge ブロックが訪問されず、すべて未到達扱いになっていた。

#### 対応内容（2025-10-07）

- `find_unreachable_blocks` を全面リライトし、以下を実施
  - ブロックラベル→ブロック本体のルックアップテーブルを事前構築
  - エントリブロックを訪問済み扱いする前に DFS へ渡し、`TermBranch` / `TermJump` / `TermSwitch` の後続を漏れなく再帰訪問
  - 未定義ラベルは解析対象外とし、`validate_cfg` 側のエラー検知に委譲
- 変更ファイル: `compiler/ocaml/src/core_ir/cfg.ml`

#### 検証結果

ネストした if を含む 118 件の既存テストを再実行し、回帰がないことを確認済み。到達不能ブロック警告が正しく動作することを確認。

---

## 更新履歴

- **2025-10-06**: 初版作成（Phase 1 完了時）
  - レコードパターン問題を記録
  - Handler パース問題を記録
  - Unicode XID 未対応を記録
- **2025-10-06**: Handler パース問題を解消し、追跡リストから除外
- **2025-10-07**: Phase 2 Week 9 更新
  - 型エラー生成順序の問題を追加（ID: 6）
  - 7件のテスト失敗を分析・文書化
  - Phase 2 後半での対応計画を策定
- **2025-10-07**: Phase 2 Week 9-10 更新（CLI統合と診断出力の改善完了後）
  - 型エラー生成順序（ID: 6）に新たな発見を追記
  - 診断層の実装は完了、問題の本質は型推論層にあることを明確化
  - 優先度を 🟡 Medium → 🟠 High に引き上げを推奨
  - 文脈依存の unify ヘルパー関数の実装案を追加
  - 対応状況トラッキング表を更新（診断層完了・型推論層未対応）
- **2025-10-07**: Phase 2 Week 10 更新
  - 型推論エンジンに文脈依存ヘルパーを導入し、診断用エラー型を完全生成
  - `dune exec -- ./tests/test_type_errors.exe` を実行し 30/30 件の成功を確認
  - 技術的負債トラッキングのステータスを「完了」に更新

- **2025-10-07**: Phase 2 完了時更新
  - Phase 2 で解消した技術的負債を「解決済み項目」へ移動（ID: 1, 2, 6, 7）
  - 配列リテラル型推論（ID: 8）を Phase 3 向け課題として追加
  - 対応状況トラッキング表を更新（Phase 2 完了状態に反映）
  - Unicode XID を「モジュール修飾対応（完了）」と「完全対応（未対応）」に分割（ID: 2, 2b）

- **2025-10-07**: Phase 3 Week 10 更新
  - CFG構築時の到達不能ブロック生成問題を追加（ID: 9）
  - ネストした制御フロー構造での問題を詳細に分析・文書化
  - 根本原因（ブロック接続の不整合・ラベル参照の不一致・線形化の順序問題）を特定
  - 回避策（到達不能ブロック警告の許容）を実装済み
  - Phase 3 Week 11-12 での対応計画を策定
  - 対応状況トラッキング表を更新（ID: 9 追加）

---

### 10. LLVM 18型付き属性のバインディング制限

**分類**: LLVM統合 / ABI実装
**優先度**: 🟡 Medium → 🟢 Low
**ステータス**: 解決済み（Phase 1-5 ランタイム連携タスクで FFI 実装完了）
**発見日**: 2025-10-09 / Phase 3 Week 14-15
**更新日**: 2025-10-10（型付き属性サポート導入）

#### 問題の詳細

LLVM 18 の ABI 属性（`sret`, `byval`）は型付き属性として実装する必要があるが、標準の llvm-ocaml バインディングは `LLVMCreateTypeAttribute` を公開していないため、Phase 3 までは文字列属性によるワークアラウンドで凌いでいた。

**影響範囲（過去）**:
- `compiler/ocaml/src/llvm_gen/abi.ml` の `add_sret_attr`, `add_byval_attr`
- 16 バイトを超える構造体戻り値・引数の ABI 処理（System V ABI）

#### 対応内容（Phase 1-5 ランタイム連携）

- `compiler/ocaml/src/llvm_gen/llvm_attr.ml` を新設し、`Llvm.enum_attr_kind` と C スタブを組み合わせて型付き属性を生成。
- `compiler/ocaml/src/llvm_gen/llvm_attr_stubs.c` から `LLVMCreateTypeAttribute` を直接呼び出し、`Llvm.llattribute` に変換。
- `add_sret_attr` / `add_byval_attr` を型付き属性ベースへ更新し、LLVM が求めるポインティ型情報を確実に伝達。未知の属性名が返された場合は既存の文字列属性にフォールバックする設計に変更。

```ocaml
let sret_attr =
  try Llvm_attr.create_sret_attr llctx ret_ty
  with Llvm.UnknownAttribute _ -> Llvm.create_string_attr llctx "sret" ""
in
Llvm.add_function_attr llvm_fn sret_attr attr_kind
```

#### 検証結果

- `opam exec -- dune build` を実行し、OCaml/C スタブを含むビルドが成功することを確認。
- `scripts/verify_llvm_ir.sh` を用いた既存の IR 検証フローで差分が生じないことを確認（byval/sret 属性が型付きで出力される）。
- `tests/llvm-ir/golden/*.ll.golden` を確認し、構造体戻り値・引数に型付き属性が付与されることを手動検証（CI のゴールデンテストも同時に通過）。

#### 今後のフォローアップ

- Windows x64 / ARM64 ABI では型付き属性の扱いが必須となるため、今回導入した `Llvm_attr` モジュールの C スタブを流用してターゲットごとの差分検証を追加する（`docs/plans/bootstrap-roadmap/2-6-windows-support.md` でトラッキング）。
- `Llvm_attr` のユニットテストを追加し、`Llvm.UnknownAttribute` 例外経路・フォールバックの挙動を固定化する（Phase 2 Medium 優先度）。

#### 記録

- 実装ファイル: `compiler/ocaml/src/llvm_gen/llvm_attr.ml`, `compiler/ocaml/src/llvm_gen/llvm_attr_stubs.c`, `compiler/ocaml/src/llvm_gen/abi.ml:170-199`
- ドキュメント: `compiler/ocaml/README.md` Phase 1-5 セクション、`docs/plans/bootstrap-roadmap/1-5-runtime-integration.md` §6.1
- 参考資料: `docs/plans/bootstrap-roadmap/1-4-llvm-targeting.md` §5

---

## Phase 1-7完了時点のサマリー（2025-10-11）

### Phase 1-7 で完了した実装

- ✅ **GitHub Actions ワークフロー構築**: `.github/workflows/bootstrap-linux.yml`
- ✅ **ローカル CI 再現スクリプト**: `scripts/ci-local.sh`
- ✅ **メトリクス記録スクリプト**: `tooling/ci/record-metrics.sh`
- ✅ **LLVM IR 検証の明示化**: llvm-as → opt -verify → llc
- ✅ **コンパイラバイナリの命名**: `remlc-ocaml --version`
- ✅ **テスト結果の JUnit XML 出力**
- ✅ **LLVM IR・Bitcode の統合アーティファクト化**
- ✅ **依存関係キャッシュの最適化**: LLVM 18, OCaml/opam

### Phase 1-7 で発見された技術的負債

- 🟡 **ID 14**: CI実行時間の最適化（Medium, Phase 2 で対応）
- 🟡 **ID 15**: メトリクス自動記録の精度向上（Medium, Phase 2 で対応）
- 🟢 **ID 16**: カバレッジレポート生成（Low, Phase 2-3 で対応）

### Phase 1-8 へ引き継ぐ技術的負債

**macOS 固有の課題**:

- **M1**: Homebrew 依存関係の管理とバージョン固定
- **M2**: Mach-O ランタイムのビルド規則整備
- **M3**: LLVM toolchain の差異検証（Linux vs macOS）
- **M4**: Xcode Command Line Tools のバージョン管理

**共通課題**:

- **C1**: CI 実行時間の最適化（Linux での知見を macOS に適用）
- **C2**: メトリクス記録の自動化（macOS セクションの追加）
- **C3**: アーティファクト管理の統一（命名規則の整合）

**詳細**: [1-7-to-1-8-handover.md](../../../docs/plans/bootstrap-roadmap/1-7-to-1-8-handover.md)

---

## Phase 3完了時点のサマリー（2025-10-09）

### 完了した技術的負債

- ✅ **ID 1**: レコードパターン複数アーム（Lexer分割で解消）
- ✅ **ID 2**: Unicode XID（モジュール修飾対応完了、完全対応はPhase 3-4）
- ✅ **ID 6**: 型エラー生成順序（文脈ヘルパー導入で解消）
- ✅ **ID 7**: Handler宣言パース（仕様準拠に更新）
- ✅ **ID 9**: CFG構築時の到達不能ブロック生成（修正完了）
- ✅ **ID 10**: LLVM 18型付き属性のバインディング制限（`Llvm_attr` FFI で解消）

### Phase 2へ引き継ぐ技術的負債

**High優先度（Week 17-20で対応）**:

- **H1**: 型マッピングのTODO解消（type_mapping.ml:75,135,186）
  - 2025-10-17: Typer 実装チーム（Type Inference / Effect 担当）へ割り当て済み。[2-2-effect-system-integration.md](../../docs/plans/bootstrap-roadmap/2-2-effect-system-integration.md) §3 で `effect_profile` 正規化と Stage 判定ヘルパを `type_inference_effect.ml` / `core_ir/effect.ml` として切り出す計画を確定。型クラス辞書との独立性検証と `Type_env.function_entry` 拡張を同フェーズで同時実施する。
- **H2**: Windows x64 ABI検証（8バイト閾値）
- **H3**: ゴールデンテストの拡充
- **H4**: CFG線形化の完成

**Medium優先度（Week 20-30で対応）**:

- **M1**: 配列リテラル型推論
- **M2**: Unicode XID完全対応（uutf/uucp統合）
- **M3-M9**: Switch文、レコード、配列、型クラス辞書、診断強化

**技術的制限**:

- （更新なし）

**詳細**: [phase3-to-phase2-handover.md](phase3-to-phase2-handover.md)

- **2025-10-10**: Phase 1-6 完了時更新
  - Phase 1-6 で発見された技術的負債を追加（ID: 11, 12, 13）
  - 統計機能の拡張（ID: 11）を Medium 優先度で追加
  - CLI 統合テストの完全な網羅（ID: 12）を Medium 優先度で追加
  - ベンチマークスイートの作成（ID: 13）を Low 優先度で追加
  - 対応状況トラッキング表を更新（Phase 1-6 完了状態に反映）

- **2025-10-11**: Phase 1-7 完了時更新
  - Phase 1-7（x86_64 Linux 検証インフラ構築）完了を記録
  - Phase 1-7 で発見された技術的負債を追加（ID: 14, 15, 16）
  - CI実行時間の最適化（ID: 14）を Medium 優先度で追加
  - メトリクス自動記録の精度向上（ID: 15）を Medium 優先度で追加
  - カバレッジレポート生成（ID: 16）を Low 優先度で追加
  - Phase 1-8（macOS プレビルド対応）へ引き継ぐ技術的負債を整理
  - 対応状況トラッキング表を更新（Phase 1-7 完了状態に反映）

- **2025-10-11**: Phase 1-8 完了時更新
  - Phase 1-8（macOS Apple Silicon ARM64 対応）完了を記録
  - Phase 1-8 で発見された技術的負債を追加（ID: 18, 19, 20）
  - Homebrew LLVM バージョン変動リスク（ID: 18）を Medium 優先度で追加
  - GitHub Actions macOS ランナーコスト（ID: 19）を Medium 優先度で追加
  - x86_64 macOS ワークフロー未実装（ID: 20）を Low 優先度で追加
  - Phase 2 へ引き継ぐ技術的負債を整理
  - 対応状況トラッキング表を更新（Phase 1-8 完了状態に反映）

---

## Phase 1-8 開始時点での発見事項（2025-10-12）

### 17. dune-project の構文エラー

**分類**: ビルドシステム / CI
**優先度**: 🔴 Critical → ✅ 解決済み
**ステータス**: 解決済み（Phase 1-8 開始直後）
**発見日**: 2025-10-12
**解決日**: 2025-10-12

#### 問題の詳細（ID 17）

GitHub Actions の Lint ステージで `dune-project` の構文エラーが発生：

```text
File "dune-project", line 26, characters 1-35:
26 |  (ocamlformat (= 0.26.2) :dev true)))
      ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
Error: Atom or quoted string expected
```

**根本原因**:

- 26行目の `(ocamlformat (= 0.26.2) :dev true)))` が不正な構文
- `:dev true` の記法が Dune 3.0 では非対応
- トップレベルの `(fmt ...)` stanza も Dune 3.0 + `(using fmt 1.5)` の組み合わせで非対応

#### 実装内容（2025-10-12）

1. **`dune-project` の修正**:
   - 26行目から `:dev true` を削除し、`(ocamlformat (= 0.26.2))` のみに変更
   - `(using fmt 1.5)` を削除（Dune 3.0 では 1.0-1.2 のみサポート）
   - トップレベルの `(fmt (version 0.26.2))` stanza を削除

2. **`.ocamlformat` ファイルの作成**:
   - `compiler/ocaml/.ocamlformat` を新規作成
   - バージョン指定: `version=0.26.2`

3. **修正後の構成**:

   ```lisp
   (lang dune 3.0)
   (name reml_ocaml)
   (using menhir 2.1)

   (package
    (name reml_ocaml)
    (depends
     ...
     (ocamlformat (= 0.26.2))))
   ```

#### 検証結果（ID 17）

- `dune-project` の構文エラーを解消
- GitHub Actions の `.github/workflows/bootstrap-linux.yml` の Lint ステージが正常に動作する準備が完了
- ocamlformat のバージョン固定（0.26.2）を `.ocamlformat` ファイルで管理

#### CI への影響（ID 17）

- **修正ファイル**:
  - `compiler/ocaml/dune-project`（26行目の修正 + `using fmt` の削除）
  - `compiler/ocaml/.ocamlformat`（新規作成）
- **GitHub Actions への影響**:
  - Lint ステージが成功するようになる
  - `opam install . --deps-only --with-test` が正常に動作する

#### 参考資料（ID 17）

- Phase 1-8 計画書 §0「Linux CI ブロッカーの解消」
- Dune ドキュメント: <https://dune.readthedocs.io/en/stable/formatting.html>

---

## Phase 1-8 完了時点での技術的負債（2025-10-11）

### ✅ Phase 1-8 で解決した項目

- **ID 17**: dune-project の構文エラー（解決済み）

### 🟡 Phase 1-8 で発見された新規技術的負債

#### 18. Homebrew LLVM のバージョン変動リスク

**分類**: CI / ツールチェーン管理
**優先度**: 🟡 Medium
**ステータス**: 未対応（Phase 2 で検討）
**発見日**: 2025-10-11 / Phase 1-8

##### 問題の詳細 (ID 18)

macOS の Homebrew で LLVM をインストールする際、`brew upgrade` により LLVM のバージョンが自動更新される可能性があります。

**影響範囲**:

- GitHub Actions macOS ランナーでのビルド失敗リスク
- ローカル開発環境での LLVM バージョン不一致
- LLVM IR の互換性問題（特に型付き属性）

**現在の緩和策**:

- `brew install llvm@18` でバージョン固定
- `brew link --force llvm@18` で明示的なリンク
- GitHub Actions キャッシュで LLVM バイナリを保持

##### 対応計画 (ID 18)

**Phase 2 Week 17-20**:

- `brew extract` による特定バージョンの固定化を検討
- または prebuilt LLVM tarball を配布して Homebrew に依存しない構成を検討
- `docs/notes/llvm-spec-status-survey.md` にバージョン管理戦略を記録

**成功基準**:

- LLVM バージョンが予期せず変更されないことを保証
- GitHub Actions と ローカル環境で LLVM バージョンが一致
- LLVM アップグレード時の影響範囲が明確

---

#### 19. GitHub Actions macOS ランナーのコスト制約

**分類**: CI / インフラ
**優先度**: 🟡 Medium
**ステータス**: 未対応（Phase 2 で検討）
**発見日**: 2025-10-11 / Phase 1-8

##### 問題の詳細 (ID 19)

GitHub Actions の macOS ランナー（`macos-14`）は Linux ランナーより実行時間が長く、無料枠の消費が早い。

**影響範囲**:

- GitHub Actions の無料枠が早期に枯渇
- CI 実行回数の制約
- 並行実行数の制限

**現在の緩和策**:

- 必要最小限のトリガー設定（push は main/develop のみ）
- キャッシュの最大活用（Homebrew, LLVM, OCaml/opam）
- 長時間実行テストは Phase 2 以降に延期

##### 対応計画 (ID 19)

**Phase 2 Week 17-20**:

- Docker イメージによる事前ビルドで実行時間を短縮
- ジョブの並列化可能性を検討
- セルフホストランナーの導入を検討（コスト削減）

**Phase 2 Week 20-30**:

- 実行時間の測定と最適化
- 目標: 全ステージ 10 分以内（キャッシュヒット時 5 分以内）

**成功基準**:

- CI 実行時間が現状より 30% 削減される
- GitHub Actions の無料枠が月次で収まる
- セルフホストランナーのコスト対効果が明確

---

#### 20. x86_64 macOS ワークフロー未実装

**分類**: CI / プラットフォームサポート
**優先度**: 🟢 Low
**ステータス**: 未対応（Phase 2 で検討）
**発見日**: 2025-10-11 / Phase 1-8

##### 問題の詳細 (ID 20)

現在の GitHub Actions ワークフローは `macos-14` (ARM64) に特化しており、Intel Mac (x86_64) 向けのワークフローがありません。

**影響範囲**:

- Intel Mac 開発者への CI サポート不足
- x86_64 ターゲットでのリグレッション検出の遅延
- ユニバーサルバイナリ生成の障壁

**現在の代替手段**:

- ローカル CI スクリプト (`scripts/ci-local.sh`) は x86_64 をサポート
- 開発者がローカルで `--arch x86_64` を使用可能
- Apple Silicon Mac から x86_64 クロスコンパイルも可能

##### 対応計画 (ID 20)

**Phase 2 Week 20-30**:

- `macos-13` (x86_64) と `macos-14` (ARM64) の並行実装を検討
- または ユニバーサルバイナリ (ARM64 + x86_64) の生成を検討
- コスト対効果を評価（x86_64 ランナーのコスト vs 開発者体験）

**Phase 3 以降**:

- ユニバーサルバイナリ対応を本格実装
- クロスコンパイル検証の強化

**成功基準**:

- Intel Mac 開発者が CI で自動検証できる
- x86_64 ターゲットでのリグレッションが早期検出される
- ユニバーサルバイナリが生成できる（Phase 3）

---

## Phase 2 へ引き継ぐ技術的負債サマリー

### 🟠 High Priority（Week 17-20 で対応）

| ID | 項目 | 優先度 | ステータス | 担当 Phase | 備考 |
|----|------|--------|-----------|-----------|------|
| H1 | 型マッピングの TODO 解消 | 🟠 High | 未対応 | Phase 2 W17-20 | Typer 実装チーム（Effect/Capability/型クラス辞書） |
| H2 | Windows x64 ABI 検証 | 🟠 High | 未対応 | Phase 2 W20-30 | 8バイト閾値の検証 |
| H3 | ゴールデンテストの拡充 | 🟠 High | 未対応 | Phase 2 W17-20 | カバレッジ 90% 目標 |
| H4 | CFG 線形化の完成 | 🟠 High | 未対応 | Phase 2 W20-30 | ブロック順序最適化 |

### 🟡 Medium Priority（Week 20-30 で対応）

| ID | 項目 | 優先度 | ステータス | 担当 Phase | 備考 |
|----|------|--------|-----------|-----------|------|
| M1 | 配列リテラル型推論 | 🟡 Medium | 未対応 | Phase 3 前半 | `[1, 2, 3]` 対応 |
| M2 | Unicode XID 完全対応 | 🟡 Medium | 未対応 | Phase 3-4 | uutf/uucp 統合 |
| 11 | 統計機能の拡張 | 🟡 Medium | 部分完了 | Phase 2 W17-30 | `--metrics` フラグ |
| 12 | CLI 統合テストの完全な網羅 | 🟡 Medium | 部分完了 | Phase 2 W17-30 | スモークテスト |
| 14 | CI 実行時間の最適化 | 🟡 Medium | 未対応 | Phase 2 W17-30 | 目標 10分以内 |
| 15 | メトリクス自動記録の精度向上 | 🟡 Medium | 部分完了 | Phase 2 W17-30 | ログ自動解析 |
| 18 | Homebrew LLVM バージョン変動リスク | 🟡 Medium | 未対応 | Phase 2 W17-20 | バージョン固定 |
| 19 | GitHub Actions macOS ランナーコスト | 🟡 Medium | 未対応 | Phase 2 W17-20 | セルフホスト検討 |

### 🟢 Low Priority（Phase 3 以降で対応）

| ID | 項目 | 優先度 | ステータス | 担当 Phase | 備考 |
|----|------|--------|-----------|-----------|------|
| 3 | AST Printer の改善 | 🟢 Low | 未対応 | Phase 3 | Pretty Print, JSON |
| 5 | 性能測定 | 🟢 Low | 未対応 | Phase 3 | ベンチマーク |
| 6 | エラー回復強化 | 🟢 Low | 未対応 | Phase 3 | 診断改善 |
| 13 | ベンチマークスイートの作成 | 🟢 Low | 未対応 | Phase 2-3 | サンプル整備済み |
| 16 | カバレッジレポート生成 | 🟢 Low | 未対応 | Phase 2-3 | bisect_ppx 導入 |
| 20 | x86_64 macOS ワークフロー未実装 | 🟢 Low | 未対応 | Phase 2 W20-30 | macos-13 並行実装 |

---

**最終更新**: 2025-10-11（Phase 1-8 完了時点）
**次回更新予定**: Phase 2 Week 20（中間レビュー時）

---

## Phase 1 完了状態

### Phase 1 全体の達成状況

✅ **Phase 1 (Parser & Frontend)**: 100% 完了
✅ **Phase 2 (Typer MVP)**: 100% 完了
✅ **Phase 3 (Core IR & LLVM)**: 100% 完了
✅ **Phase 1-5 (ランタイム連携)**: 100% 完了
✅ **Phase 1-6 (開発者体験整備)**: 100% 完了
✅ **Phase 1-7 (Linux 検証インフラ)**: 100% 完了
✅ **Phase 1-8 (macOS プレビルド対応)**: 100% 完了

### Phase 2 移行準備状況

- ✅ 全報告書作成完了
- ✅ 技術的負債リスト更新完了
- ✅ 引き継ぎドキュメント作成完了
- ✅ CI/CD インフラ整備完了
- ✅ テストスイート整備完了
- ✅ ドキュメント整備完了

**Phase 2 開始準備完了** 🚀
