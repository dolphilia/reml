# クロスコンパイル仕様導入に向けた調査メモ

> 目的: Reml にクロスコンパイル機能を導入するための基礎調査と設計方針を整理し、今後の仕様策定に向けた土台を用意する。

## 1. 現状整理（2024年ドラフト時点）

- `remlc --target` は [README.md](../spec/README.md) および [guides/portability.md](../guides/portability.md) で言及済みだが、ターゲットプロファイルや標準ライブラリ配布形態は未確定。
- `RunConfig.extensions["target"]` と `@cfg` は [1-1-syntax.md](../spec/1-1-syntax.md)・[3-10-core-env.md](../spec/3-10-core-env.md)・[3-8-core-runtime-capability.md](../spec/3-8-core-runtime-capability.md) で個別に定義されているが、クロスビルド時の一貫した初期化手順が欠落。
- ランタイム/FFI の ABI 設定は [3-9-core-async-ffi-unsafe.md](../spec/3-9-core-async-ffi-unsafe.md) や [guides/llvm-integration-notes.md](../guides/llvm-integration-notes.md) でターゲット依存項目を列挙するのみで、ビルド成果物と連動する仕組みが存在しない。
- 標準ライブラリ `Core.*` はポータビリティを意識した API 定義を進めている（例: [3-5-core-io-path.md](../spec/3-5-core-io-path.md), [3-10-core-env.md](../spec/3-10-core-env.md)）が、ターゲット別ビルド成果物の配布や検証フローはこれから。
- エコシステム章（Chapter 4）は CLI・レジストリ等のドラフト段階で、クロスコンパイルに関する記述はない（[4-1-package-manager-cli.md](../spec/4-1-package-manager-cli.md), [4-3-developer-toolchain.md](../spec/4-3-developer-toolchain.md) 参照）。

## 2. クロスコンパイル調査サマリ

### 2.1 現代的なクロスコンパイル手法のパターン

- **ターゲットプロファイル管理**: Rust (`target.json`)、Zig（`builtin.Target`）、Go（`GOOS/GOARCH`）等が採用。ライブラリとランタイムをターゲット三つ組（OS/アーキテクチャ/ABI）で束ねる。
- **ツールチェーン分離**: クロスリンク/アセンブル用にターゲット専用の binutils・libc・sysroot を準備（Clang/LLVM, Rustup toolchain 方式）。
- **標準ライブラリの事前ビルド**: Rustup の `std`、Go の `pkg/GOOS_GOARCH` のようにターゲット別の標準ライブラリバイナリを配布。
- **設定駆動のビルドマトリクス**: CI ではターゲット配列を宣言し、各ターゲットで共通のビルド/テスト手順を適用（GitHub Actions の matrix, Buildkite の pipeline）。
- **エミュレーション/リモート実行**: クロスビルド後に QEMU/Wasmtime/リモートホストで smoke test を実施して互換性を検証。
- **署名付きアーティファクト**: 配布物にメタデータ（ターゲット, ハッシュ, ABI レベルなど）を付帯し、レジストリやパッケージマネージャで検証。

### 2.2 ベストプラクティスと注意点

- **ターゲット検証の一貫性**: コンパイラが吐くメタデータと CLI/実行時が参照する `RunConfigTarget` を一致させる。
- **明示的な機能フラグ**: SIMD・Packrat 等の最適化は Capability で制御し、安全なフォールバックを要求。
- **依存解決の再現性**: クロスビルド時もホストとターゲットの依存が分離されたロックファイル（例: Cargo.lock, pnpm-lock）を利用。
- **ABI 差異の吸収層**: 呼出規約やデータレイアウトの違いを中間層で統一し、ユーザーコードを守る。

## 3. Reml 指針（[0-1-project-purpose.md](../spec/0-1-project-purpose.md)）との照合

- **性能 (1.1)**: ターゲットごとの標準ライブラリを事前ビルドし、クロスコンパイル時に再コンパイルを避けることで線形時間を維持。ビルドキャッシュと差分配布を前提にする。
- **安全性 (1.2)**: 標準ライブラリとランタイムの組み合わせを署名付きメタデータで検証し、ABI 不一致はビルド時点で停止。`@cfg` 分岐は `RunConfigTarget` を通じて型安全に評価。
- **書きやすさ (2.1)**: `reml target list/show` などの CLI を用意し、主要ターゲットはプリセットで選択可能にする。ターゲット JSON の自由度は保持しつつも、典型構成は宣言的に選べるようにする。
- **エラー品質 (2.2)**: クロスビルド専用の Diagnostic コード（例: `target.profile.missing`, `target.abi.mismatch`）を追加し、原因と対応策を CLI が提示する。
- **Unicode/プラットフォーム整合 (3.1, 3.2)**: `Core.Env.infer_target_from_env` を拡張して Unicode 正規化ポリシー差異やファイルシステム機能の可用性を Capability として報告し、DSL との統合を崩さない。

## 4. 推奨する仕様骨子

### 4.1 ターゲットプロファイルとメタデータ

- `TargetProfile`（仮称）を `4.x` 章に新設。`os`, `family`, `arch`, `abi`, `vendor`, `features`, `capabilities`, `stdlib_version`, `runtime_revision` 等を保持。
- プロファイルは TOML/JSON 両対応とし、CLI コマンドで生成/検証 (`reml target scaffold`, `reml target validate`) を提供。
- レジストリ側メタデータと互換にし、将来の `reml publish` がターゲット情報を埋め込めるようにする。

### 4.2 標準ライブラリ & ランタイムのバンドル戦略

- `Core.*` のビルド成果物をターゲットごとに `artifact/std/<triple>/<hash>` へ格納し、CLI がクロスビルド時に自動解決。
- ランタイム（GC/RC/FFI シム）は ABI ごとに `runtime/<profile>` を用意し、バイナリ互換を `runtime_revision` で表現。
- `reml toolchain install <profile>` で必要なライブラリ・ランタイム・サードパーティ依存をまとめて取得できるよう設計。

### 4.3 ツールチェーンと検証フロー

- `reml build --target <profile>` が次を実行するよう仕様化:
  1. プロファイル検証 (`TargetProfile` → `RunConfigTarget` 変換、`Core.Env` への注入)。
  2. 標準ライブラリ/ランタイムの互換性チェック（署名とハッシュ）。
  3. LLVM backend へのターゲット指定（triple, data layout, CPU/feature string）。
  4. 後処理: クロスリンカ呼び出し、成果物メタデータ生成 (`.remlpkg` 等)。
- `reml test --target` は QEMU 等のエミュレータ/リモート実行設定を `TargetProfile` に紐づけ、Smoke Test を自動化する設計を想定。

### 4.4 設定・診断の拡張

- `RunConfig.extensions["target"]` に `profile_id`, `diagnostics`, `capability_overrides` を追加し、IDE/LSP へターゲット情報を配信。
- `Core.Diagnostics` に `DiagnosticDomain::Target` を追加し、クロスコンパイル関連のメッセージキーを整理。
- `Core.Runtime` の Capability Registry に `TargetCapability` グループを追加し、`has_capability` の結果を `TargetProfile` 由来で初期化。

### 4.5 レジストリ / パッケージ連携

- [4-2-registry-distribution.md](../spec/4-2-registry-distribution.md) と連携し、パッケージのメタデータに `targets = ["x86_64-unknown-linux-gnu", ...]` を追加。
- `reml publish` はクロスビルド成果物を添付する場合、ターゲット毎にハッシュ+署名+互換バージョン範囲を記録。
- `reml add` は互換ターゲットが存在しない場合に警告または失敗とし、`--allow-source-build` でソースビルドへフォールバック。

## 5. 今後のタスク提案

1. `TargetProfile` ドラフト仕様の執筆（4.x 章に節を追加）。
2. CLI 仕様（[4-1-package-manager-cli.md](../spec/4-1-package-manager-cli.md)）へターゲット管理サブコマンドの追記。
3. `Core.Env` / `Core.Runtime` / `Core.Diagnostics` への必要拡張を Chapter 3 ドキュメントに反映。
4. レジストリ仕様（[4-2-registry-distribution.md](../spec/4-2-registry-distribution.md)）へターゲットメタデータ規定を追加。
5. 将来のセルフホスト計画（[guides/llvm-integration-notes.md](../guides/llvm-integration-notes.md)）と整合するビルドパイプライン図を作成し、クロスリンカ/エミュレーション手順を別途まとめる。

---

このメモはクロスコンパイル仕様を正式文書化する前段階の参照資料として利用し、各章のドラフトを更新する際の共通ベースラインとして扱うことを想定している。
