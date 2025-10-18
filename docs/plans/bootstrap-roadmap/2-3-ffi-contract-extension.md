# 2.3 FFI 契約拡張計画

## 目的
- Phase 2 で [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) に定義された ABI/所有権契約を OCaml 実装へ反映し、x86_64 Linux (System V)、Windows x64 (MSVC)、Apple Silicon macOS (arm64-apple-darwin) の 3 ターゲットでブリッジコードを検証する。
- `AuditEnvelope` に FFI 呼び出しのメタデータを記録し、診断と監査の一貫性を確保する。

## スコープ
- **含む**: FFI 宣言構文の Parser 拡張、Typer による ABI/所有権チェック、ブリッジコード生成、ターゲット別（Linux x86_64 / Windows x64 / macOS arm64）ビルド、監査ログ拡張。
- **含まない**: 非同期ランタイム実装の刷新、プラグイン経由の FFI 自動生成。これらは Phase 3 以降。
- **前提**: Phase 1 のランタイム連携が完成し、Phase 2 の効果システム統合と衝突しない設計であること。Apple Silicon 対応については [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) および [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md) に整備済みの計測・CI 手順を踏襲する。

## 作業ディレクトリ
- `compiler/ocaml/src/parser`, `compiler/ocaml/src/typer` : FFI 宣言解析と型検証
- `compiler/ocaml/src/codegen` : ブリッジコード生成、ABI 設定
- `runtime/native` : 所有権ヘルパ・FFI スタブ
- `tooling/ci`, `tooling/ci/macos`, `tooling/runtime/capabilities` : Linux/Windows/macOS 向けブリッジ検証と Capability ステージ管理
- `docs/spec/3-9-core-async-ffi-unsafe.md`, `docs/notes/llvm-spec-status-survey.md`, `docs/plans/bootstrap-roadmap/1-8-macos-prebuild-support.md` : 契約・測定・macOS 支援資料

## 作業ブレークダウン

## 進捗トラッキング（2025-10 時点）

| 作業ブロック | ステータス | 完了済み項目 | 次のステップ |
| --- | --- | --- | --- |
| 前提確認・計画調整 | **進行中** | 2025-10-18 に `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` を再実行し、`reports/runtime-capabilities-validation.json` （timestamp `2025-10-18T03:23:33.958135+00:00`）を更新。override 追加草案メモと `reports/ffi-macos-summary.md` の初回記録を追記済み。 | override 追加のドラフト PR を作成し、再実行ログと `reports/ffi-macos-summary.md` にまとめた CI ログ断片をレビューコメントへ添付。 |
| 1. ABI モデル設計 | **進行中** | Darwin 計測計画を `docs/notes/llvm-spec-status-survey.md` に追記し、計測ログ用テンプレート `reports/ffi-macos-summary.md` を作成。 | OCaml 側 ABI データ型ドラフトと計測スクリプト実行結果（DataLayout・callconv）をテンプレートへ記録。 |
| 2. Parser / AST 拡張 | **進行中** | `extern_metadata` PoC を実装し、`@ffi_target` などの属性を抽出するメタデータとユニットテストを追加。2025-10-18 時点で `extern_decl.extern_target` を `extern_block_target` に改名し、ブロック単位のターゲット整合ロジックを確定。 | Typer へのメタデータ伝播と CLI/監査診断への接続方針を整理し、ゴールデン出力の更新計画を立案。 |
| 3. Typer 統合と ABI 検証 | **未着手** | — | FFI 型ホワイトリストと所有権検証の設計メモを起案。 |
| 4. ブリッジコード生成 | **未着手** | — | ターゲット別 stub 生成ロジックの責務分担を `codegen` / `runtime/native` チームと摺り合わせ。 |
| 5. 監査ログ統合 | **準備中** | `AuditEnvelope.metadata.bridge.*` のキー案と effect-system 設計ノートの TODO を同期。`effect.stage.iterator.source_detail` を追加し、Iterator audit のログにソース詳細を保持。 | JSON スキーマ案のドラフト作成と、`iterator_audit` との突合ルール整理。 |
| 6. プラットフォーム別テスト | **準備中** | Apple Silicon 実行計画を `docs/notes/llvm-spec-status-survey.md` の計測タスク・新規レポートにリンク。`scripts/ci-local.sh --stage beta` で Build/LLVM IR 検証まで完走し、測定ログ雛形を更新。 | Apple Silicon 実機／ランナーでの最小 FFI サンプル実行計画を策定し、必要な Homebrew ツールチェーン確認。成果をもとに `reports/ffi-macos-summary.md` の未実施テスト項目を消化。 |
| 7. ランタイム連携とテスト | **未着手** | — | FFI ヘルパ API の拡張方針を `runtime/native` ドキュメントに追記するドラフトを準備。 |
| 8. ドキュメント更新と引き継ぎ | **進行中** | Apple Silicon 対応更新に加え、計測テンプレートと override 提案を関連資料へリンク。`reports/ffi-macos-summary.md` に最新ログ（ステージ指定実行結果、LLVM IR 単体検証）を追記。 | 実装着手後に更新するべき仕様・ガイドのチェックリストを作成。監査ゴールデン更新と合わせて完了レポート案を整理。 |

### 最新進捗サマリー（2025-10-18）

- `scripts/ci-local.sh` に `--stage` オプションを追加し、`REMLC_EFFECT_STAGE` の CLI 指定を可能化。macOS arm64 環境で Build / LLVM IR 検証まで完走するルートを確認。
- `extern_decl` のターゲット集約フィールドを `extern_block_target` へ改名し、Parser・AST Printer・設計ドキュメントを同期。属性由来のターゲット情報は `extern_metadata` に集約され、ブロックレベルの整合チェックを準備できた。
- `iterator` 監査メタデータに `effect.stage.iterator.source_detail` を追加し、Typer で算出した `iterator_source` を JSON に保持。将来の FFI 監査キーと同一フォーマットで扱えるよう設計。

### 残タスクと次のステップ

1. **ゴールデン更新とテスト再実行**
   - `compiler/ocaml/tests/golden/audit/effects-residual.jsonl.golden` を `effect.stage.runtime` の最新出力に合わせて更新。
   - `dune fmt` 差分を解消し、`scripts/ci-local.sh` の Lint / Test ステップをスキップなしで通過させる。
2. **Typer 統合タスクの具体化**
   - `extern_metadata` → `AuditEnvelope.metadata.bridge.*` の写像設計を Issue 下書き（計画内チェックリスト 1〜4）として登録。
   - 所有権・ABI 検証ロジックに必要な型や診断コード（`ffi.contract.*`）のスケルトンを用意。
3. **監査スキーマと CI 連携**
   - JSON スキーマ案（`bridge` オブジェクト）をドラフト化し、`tooling/runtime/audit-schema.json` で検証。
   - `tooling/ci/collect-iterator-audit-metrics.py` に FFI ブリッジ関連指標を追加し、`ffi_bridge.audit_pass_rate`（仮）の算出フローをレビューに付す。
4. **プラットフォームテスト推進**
   - macOS arm64 での FFI サンプル実装／ABI 検証ケース（可変長、構造体戻りなど）を着手し、`reports/ffi-macos-summary.md` の未実施項目を埋める。
   - 同報告書に Windows / Linux との比較観点を追記し、クロスプラットフォーム整合チェックの計画を整備。

### 2025-10-18 ログ・測定サマリー

- **`scripts/validate-runtime-capabilities.sh`**: `tooling/runtime/capabilities/default.json` を対象に再実行し、`reports/runtime-capabilities-validation.json` の timestamp を `2025-10-18T03:23:33.958135+00:00` へ更新。`arm64-apple-darwin` override が `runtime_candidates` に出力されること、および `validation.status = ok` を確認済み。
- **`scripts/ci-local.sh --target macos --arch arm64 --stage beta`**: Lint → Build → Test → LLVM IR → Runtime（AddressSanitizer 含む）まで完走。生成された LLVM IR は `/tmp/reml-ci-local-llvm-ir-5983`、詳細ログは `reports/ffi-macos-summary.md` §2 に追記。
- **`compiler/ocaml/scripts/verify_llvm_ir.sh --target arm64-apple-darwin compiler/ocaml/tests/llvm-ir/golden/basic_arithmetic.ll`**: 追加で単体検証を実行し、LLVM 18.1.8 で `.ll → .bc → .o` パイプラインの成功を確認。生成物パスは `reports/ffi-macos-summary.md` §2 に追記。
- **フォローアップ**: (1) `effects-residual.jsonl.golden` を含む監査ゴールデンを Stage Trace 仕様に合わせて更新。（2）`ci-local` のテストステップ常時実行に備えて、`dune fmt` 差分の解消タスクを Phase 2-3 backlog に登録。

### Typer `extern_metadata` 設計メモ（ドラフト）

- Parser で抽出済みの `extern_metadata` を Typer へ伝搬し、以下のキーを `AuditEnvelope.metadata.bridge.*` に写像する：`bridge.target`（例: `arm64-apple-darwin`）、`bridge.arch`（`arm64` / `x86_64`）、`bridge.abi`（`system_v` / `msvc` / `darwin_aapcs64`）、`bridge.ownership`（`borrowed` / `transferred` / `reference`）、`bridge.extern_symbol`（リンク先シンボル名）、必要に応じて `bridge.alias`・`bridge.library` を追加。`extern_decl` 側の集約フィールドは `extern_block_target` へ改名済みで、Typer ではブロック既定値とアイテム個別値の整合を照合する。
- Typer では `extern_metadata` の欠落・矛盾を `ffi.contract.*` 診断として報告し、CLI JSON と監査ログで同一内容を表示する。`RuntimeCapabilityResolver` が提供する `stage_trace` に FFI 情報を追記し、効果診断と整合したメタデータを構築する。
- Runtime 側が追記する `bridge.callsite`（モジュール/関数）と整合させるため、Typer で `bridge.symbol_path` を計算したうえで `AuditEnvelope` へ渡す。

#### Issue 下書き案（Typer: extern_metadata パイプライン）

1. **AST → Typer のデータ受け渡し**  
   `typed_ast.ml` に extern 解析結果を格納するレコードを追加し、`extern_metadata` を必須フィールドとして保持。`bridge.target` 未指定時は Capability JSON のデフォルトターゲットを補完する。
2. **所有権と ABI の検証ロジック実装**  
   `type_inference.ml` に `check_extern_bridge_contract`（仮）を実装し、許可されていない所有権/ABI 組合せを検出。失敗時は `ffi.contract.ownership_mismatch` / `ffi.contract.unsupported_abi` 診断を新設。
3. **`AuditEnvelope` 拡張**  
   `audit_envelope.ml` に `bridge` サブレコードを追加し、Typer が JSON 生成に必要なキーを設定。effect 系メタデータと共通のフォーマッタを利用できるよう `AuditEnvelope.Metadata` を整理。
4. **ゴールデンテスト更新**  
   `compiler/ocaml/tests/golden/audit/ffi_target.json.golden` を新規追加し、`arm64-apple-darwin` と `x86_64-pc-windows-msvc` の 2 ケースを固定。CLI JSON ゴールデンにも FFI 診断を追加し、残存効果診断との併用ケースを検証する。

### JSON 監査スキーマ更新案（`ffi_target` 拡張）

- `AuditEnvelope` スキーマに `bridge` オブジェクトを追加し、必須プロパティとして `bridge.target` / `bridge.arch` / `bridge.abi` / `bridge.ownership` / `bridge.extern_symbol` を定義。オプションで `bridge.alias`, `bridge.library`, `bridge.callconv`, `bridge.audit_stage` を許容する。
- スキーマ改訂は `tooling/runtime/audit-schema.json`（ドラフト）で管理し、検証スクリプトに `./scripts/validate-runtime-capabilities.sh --schema audit` を追加して自動チェックする案を提案。2025-10-18 時点でドラフト v0 を追加し、`diagnostics[]` 配列や `bridge.*` 必須キーを定義済み。
- ゴールデンテスト: `compiler/ocaml/tests/golden/audit/ffi_target.json.golden` を新設し、`ffi_target = arm64-apple-darwin`／`ffi_target = x86_64-pc-windows-msvc` のサンプルを記録。`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に `ffi.bridge.audit_pass_rate` 指標を追記する。
- レビュー体制: 一次レビューは Diagnostics チーム、二次レビューは FFI チーム、最終承認は `tooling/ci` チーム（CI ゲート整合確認）。週次スタンドアップで進捗共有し、採択前に `reports/ffi-macos-summary.md` のサンプルを提示する。
- スクリプト更新: `tooling/ci/collect-iterator-audit-metrics.py` に FFI ブリッジ指標 `ffi_bridge.audit_pass_rate` を追加済み。出力 JSON は後方互換性のため iterator 指標をトップレベルに残したまま、`metrics[]` 配列と `ffi_bridge` サマリーを併記する。

### Capability override 提案（arm64-apple-darwin）

- ステージ案: `beta`（Phase 2-3 で FFI 契約と診断が安定するまで安定版から分離）
- 追加 Capability 候補: `ffi.bridge`, `process.spawn`（Windows x64 と同一セットで開始し、macOS 固有 Capability は Phase 2-3 後半で再評価）
- 検証手順案:
  - `scripts/validate-runtime-capabilities.sh tooling/runtime/capabilities/default.json` を実行し、`runtime_candidates` に `arm64-apple-darwin` を追加。
  - Apple Silicon ランナーで `scripts/ci-local.sh --target macos --arch arm64 --stage beta` を実行し、`iterator.stage.audit_pass_rate` が 1.0 であることを確認。
  - 監査ログ: `reports/ffi-macos-summary.md` に呼出規約検証結果と Capability stage 差分を記録し、`docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md` に記載するメトリクス更新案と合わせてレビュー依頼を出す。
- レビューコメント草案（ドラフト）: 「`tooling/runtime/capabilities/default.json` に `arm64-apple-darwin` override を追加し、ステージは `beta` で開始。Capability は既存 Windows beta と同じ `ffi.bridge` / `process.spawn` を割り当て、Phase 2-3 期間中に macOS 固有 Capability を精査する。追加後に `scripts/validate-runtime-capabilities.sh` と `scripts/ci-local.sh --target macos --arch arm64 --stage beta` を実行してレポートを共有する。」

## 直近アクション（次の 2 週間）

- `tooling/runtime/capabilities/default.json` への `arm64-apple-darwin` override 変更を PR 化し、`scripts/validate-runtime-capabilities.sh` 再実行ログと `reports/runtime-capabilities-validation.json` の差分を添付してレビュー提出。
- `scripts/ci-local.sh --target macos --arch arm64 --stage beta` を実行し、`reports/ffi-macos-summary.md` に初回計測値とログ（IR/ABI 検証・監査サマリー）を記録する。
- Typer 側で `extern_metadata` を読み取り、所有権・ターゲット情報を `AuditEnvelope.metadata.bridge.*` へ渡す設計メモとタスク分解（issue 下書き）を準備する。
- JSON 監査スキーマ更新案とゴールデンテスト拡張（`ffi_target` サンプル）をまとめ、効果診断チームとのレビュー体制を確定する。

### 1. ABI モデル設計と仕様整理（29-30週目）
**担当領域**: FFI 基盤設計

1.1. **ABI 仕様の抽出**
- [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) の ABI テーブルを OCaml データ型に写像
- System V ABI (x86_64 Linux)、MSVC ABI (x86_64 Windows)、AAPCS64/Darwin ABI (arm64 macOS) の差分整理
- 呼出規約（calling convention）の形式化
- 構造体レイアウト・アライメントルールの定義（Darwin 固有のレイアウト差分は [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) と突合）

1.2. **所有権契約の設計**
- `docs/notes/llvm-spec-status-survey.md` §2.4 の RC 契約を OCaml データ構造化（`Ownership::Transferred`/`Borrowed` 等）
- FFI 境界での所有権移転ルール
- メモリ安全性の検証ポリシー
- `effect {ffi}`/`effect {unsafe}` 境界との連携

1.3. **ターゲット設定システム**
- ターゲット別の ABI 設定テーブル（`x86_64-unknown-linux-gnu` / `x86_64-pc-windows-msvc` / `arm64-apple-darwin`）
- コンパイル時のターゲット切替ロジック
- `--target` フラグと `--arch` の整合処理（`scripts/ci-local.sh` の引数設計を参照）
- Phase 2 型クラス・効果との統合方針

**成果物**: ABI データモデル、所有権設計、ターゲット設定

### 2. Parser/AST 拡張（30週目）
**担当領域**: FFI 構文解析

2.1. **FFI 宣言構文の実装**
- `extern "C"` ヘッダおよび複数宣言ブロックの構文（1-1 §B.4）
- ライブラリ指定や名前マングリングのオプションを既存仕様の拡張ポイント（コメント/属性）として扱い、新構文を導入しない
- 所有権契約はシグネチャ付随メタデータとして格納（構文レイヤでは属性追加を行わない）

2.2. **AST ノード拡張**
- `Decl::Extern` ノードの追加
- ターゲットトリプル/呼出規約/所有権メタデータを保持するフィールド
- Span 情報の保持
- デバッグ用の AST pretty printer 更新

2.3. **パーサテスト整備**
- FFI 宣言の正常系テスト
- ABI/所有権注釈のエラーケース
- ゴールデンテスト（AST 出力）
- Phase 1 パーサとの統合検証

**成果物**: 拡張 Parser、FFI AST、パーサテスト

### 3. Typer 統合と ABI 検証（30-31週目）
**担当領域**: 型検証と整合性チェック

3.1. **FFI 型の検証**
- FFI 境界で許可される型のホワイトリスト
- ポインタ型・参照型の検証
- 構造体レイアウトの互換性チェック
- 型サイズ・アライメントの計算

3.2. **所有権注釈の検証**
- 所有権の整合性チェック
- Unsafe ブロックの必要性判定
- 所有権違反の検出とエラー報告
- 借用規則の FFI への適用

3.3. **ABI 整合性チェック**
- ターゲット別の ABI ルール適用
- 呼出規約の検証
- 名前マングリングの生成
- Phase 2 効果システムとの連携（`effect {ffi}`, `effect {unsafe}` による契約確認）

**成果物**: FFI 型検証、所有権チェック、ABI 検証

### 4. ブリッジコード生成（31-32週目）
**担当領域**: コード生成

4.1. **Stub 生成ロジック**
- ターゲット別の stub テンプレート（Linux System V / Windows MSVC / macOS arm64）
- 引数マーシャリング（Reml 型 → C 型）
- 戻り値マーシャリング（C 型 → Reml 型）
- エラーハンドリング（NULL チェック等）

4.2. **LLVM IR への lowering**
- FFI 関数宣言の LLVM IR 生成
- 呼出規約の LLVM 属性への変換（`cc 10` 等、arm64-apple-darwin 固有の指定も含む）
- 構造体レイアウトの LLVM 型への写像
- デバッグ情報の保持

4.3. **C ヘッダ生成の検討**
- `cbindgen` 等のツール調査
- Reml 型から C ヘッダへの自動生成
- ライセンス・再現性の確認
- Phase 3 での本格導入の方針決定

**成果物**: Stub 生成、LLVM lowering、ヘッダ生成調査

### 5. 監査ログ統合（32週目）
**担当領域**: 診断と監査

5.1. **FFI メタデータの記録**
- `AuditEnvelope.metadata` に `bridge.stage.*`・`bridge.platform`・`bridge.abi` を追加
- ABI 種別・所有権注釈の記録
- FFI 呼び出しのトレース情報（ターゲットトリプルとアーキテクチャを含める）
- Phase 2 診断タスクとの連携

5.2. **診断メッセージの実装**
- FFI 型エラーの詳細メッセージ
- 所有権違反の説明と修正提案
- ABI ミスマッチの検出とレポート
- [3-6-core-diagnostics-audit.md](../../spec/3-6-core-diagnostics-audit.md) との整合

5.3. **監査ログの出力**
- `--emit-audit` での FFI 情報出力
- JSON スキーマの定義
- CI でのスキーマ検証
- `0-3-audit-and-metrics.md` への記録

**成果物**: FFI 監査ログ、診断メッセージ、スキーマ

### 6. プラットフォーム別テスト（32-33週目）
**担当領域**: クロスプラットフォーム検証

6.1. **Linux x86_64 テスト**
- System V ABI のサンプル FFI 呼び出し
- libc 関数の呼び出しテスト（`printf`, `malloc` 等）
- 構造体渡し・戻りのテスト
- 所有権注釈の検証テスト

6.2. **Windows x64 テスト**
- MSVC ABI のサンプル FFI 呼び出し
- Windows API の呼び出しテスト（`MessageBoxW` 等）
- ABI 差分の動作検証
- Phase 2 Windows タスクとの連携

6.3. **macOS arm64 テスト**
- Apple Silicon (arm64-apple-darwin) 上での FFI 呼び出し検証（`libSystem` / `dispatch` API など）
- Mach-O 向けスタブ生成と `codesign --verify` の簡易チェック
- Darwin ABI 固有のシグネチャ（構造体戻り値、可変長引数）の検証
- Phase 1-8 macOS 計測値と比較し、差分を `reports/ffi-macos-summary.md`（新規）へ記録

6.4. **CI/CD 統合**
- GitHub Actions に FFI テストジョブ追加
- Linux/Windows/macOS の 3 ターゲットでのテスト実行
- テストカバレッジの計測（>75%）
- ビルド時間の監視

**成果物**: プラットフォーム別テスト、CI 設定

### 7. ランタイム連携とテスト（33週目）
**担当領域**: ランタイム統合

7.1. **ランタイム C コードの拡張**
- FFI ヘルパー関数の実装
- メモリ管理の FFI 対応
- エラーハンドリングの統一
- Phase 1 ランタイムとの統合

7.2. **統合テスト**
- Reml → FFI → C → Reml のラウンドトリップ
- 複雑な構造体の受け渡し
- コールバック関数の検証
- メモリリークの検出（valgrind）

7.3. **性能計測**
- FFI 呼び出しのオーバーヘッド測定
- マーシャリングコストの評価
- `0-3-audit-and-metrics.md` への記録
- 最適化機会の特定

**成果物**: ランタイム拡張、統合テスト、性能計測

### 8. ドキュメント更新と引き継ぎ（33-34週目）
**担当領域**: 仕様整合と引き継ぎ

8.1. **仕様書フィードバック**
- [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md) への実装差分の反映
- ABI 差分の詳細化
- 所有権契約の擬似コードを追加
- 新規サンプルコードの追加

8.2. **ガイド更新**
- `docs/guides/runtime-bridges.md` の FFI セクション更新
- プラットフォーム別の注意事項を追記
- cbindgen 等のツール使用例
- トラブルシューティング情報

8.3. **Phase 3 準備**
- FFI のセルフホスト移植計画
- 残存課題の `docs/notes/` への記録
- 非同期 FFI の将来設計検討
- メトリクスの CI レポート化

**成果物**: 更新仕様書、ガイド、引き継ぎ文書

## 成果物と検証
- 3 ターゲットすべてで FFI サンプルが成功し、所有権違反時に診断が出力される。
- `AuditEnvelope` に FFI 呼び出しのトレースが追加され、`0-3-audit-and-metrics.md` で確認できる。
- 仕様ドキュメントの更新がレビュー済みで、記録が残る。

## リスクとフォローアップ
- Windows (MSVC) / macOS (Darwin) の呼出規約差異によりバグが潜む恐れがあるため、`2-6-windows-support.md` と [1-8-macos-prebuild-support.md](1-8-macos-prebuild-support.md) と連携してテストケースを共有。
- 所有権注釈の表現力が不足している場合、Phase 3 で DSL 拡張を検討する。
- FFI ブリッジ生成に外部ツールを使う場合はライセンス・再現性を `0-3-audit-and-metrics.md` に記録。

## 参考資料
- [2-0-phase2-stabilization.md](2-0-phase2-stabilization.md)
- [3-9-core-async-ffi-unsafe.md](../../spec/3-9-core-async-ffi-unsafe.md)
- [guides/runtime-bridges.md](../../guides/runtime-bridges.md)
- [0-3-audit-and-metrics.md](0-3-audit-and-metrics.md)
- [0-4-risk-handling.md](0-4-risk-handling.md)
