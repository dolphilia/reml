# 0.1 Reml 仕様書概要

## Reml の位置付け

Reml (Readable & Expressive Meta Language) は、パーサーコンビネーターを核に据えた言語設計プロジェクトです。短い記述で読みやすい構文、静的保証と高品質な診断、Unicode を前提とした国際化対応を重視し、実装者と利用者の双方が一貫したモデルで開発できることを目標にしています。詳細な設計ゴールや実装補遺は [Reml 設計ゴールと実装補遺](../notes/language/reml-design-goals-and-appendix.md) にまとめています。

## 概要の読み方

本概要は仕様書全体の地図として、各チャプターで扱う領域と相互関係を紹介します。章ごとの詳細な概要はそれぞれの `x.0` ページに集約されているため、関心のある分野に応じて参照してください。

## チャプターガイド

### Chapter 0 — 序論と目的

- 本ページ: プロジェクト全体像と各章の役割を示します。
- [0-1-project-purpose.md](0-1-project-purpose.md): Reml を設計する目的、ターゲット、ロードマップ上の位置付けを整理しています。

### Chapter 1 — 言語コア仕様

- [1-0-language-core-overview.md](1-0-language-core-overview.md): 構文・型推論・効果システム・Unicode モデルといったコア仕様の相互関係を要約しています。
- 1.1〜1.5: 字句/宣言/式、型と推論、効果安全、文字モデル、形式文法を定義し、言語処理系の最小要素を確立します。効果構文は Phase 2-5 時点で PoC として提供され、残余効果計測 (`Σ_before`/`Σ_after`) と CI 指標は [`EFFECT-002` Step4](../plans/bootstrap-roadmap/2-5-proposals/EFFECT-002-proposal.md#4-診断・ci-計測整備week33-day1-2) および `docs/notes/effects/effect-system-tracking.md` に従って運用されます。

### Chapter 2 — 標準パーサー API

- [2-0-parser-api-overview.md](2-0-parser-api-overview.md): `Core.Parse` 系 API 全体の構成と目的を整理し、入力モデルからエラー設計までの流れを俯瞰します。
- 2.1〜2.6: パーサ型、コンビネーター、字句レイヤ、演算子ビルダー、エラー、実行戦略を段階的に解説し、Reml の構文処理基盤を提供します。

### Chapter 3 — 標準ライブラリ

- [3-0-core-library-overview.md](3-0-core-library-overview.md): `Core.*` モジュール群の役割をまとめ、プレリュードから環境機能までの接続点を示します。
- 3.1〜3.10: 反復・コレクション・テキスト・数値/時間・IO/Path・診断/監査・設定/データ・ランタイム/Capability・非同期/FFI/Unsafe・環境統合をカバーします。

### Chapter 4 — エコシステム仕様（ドラフト）

- [4-0-ecosystem-overview.md](4-0-ecosystem-overview.md): CLI・レジストリ・ツールチェーン・コミュニティ・指標管理・ガバナンスの全体像を整理します。
- 4.1〜4.6: パッケージ管理と CLI、レジストリ配布、開発ツールチェーン、コミュニティ/コンテンツ戦略、ロードマップ指標、リスクとガバナンスを定義します。

### Chapter 5 — 公式プラグイン仕様（ドラフト）

- [5-0-official-plugins-overview.md](5-0-official-plugins-overview.md): 公式 Capability プラグインの設計指針と Runtime/監査との整合を俯瞰します。
- 5.1〜5.7: システムコール、プロセス/スレッド、仮想メモリ、シグナル、ハードウェア情報、リアルタイム機能、DSL プラグイン契約（仕様化済みの `Core.Parse.Plugin`）を扱い、プラットフォーム統合と DSL 拡張時の契約を詳細化します。

## 最近の統合ハイライト

- `../guides/core-parse-streaming.md` で定義していた `DemandHint`／`FlowController` API を [2-7-core-parse-streaming.md](2-7-core-parse-streaming.md) に統合し、実行戦略（2-6）との連携を仕様内で完結させました。
- `../guides/data-model-reference.md` の QualityReport JSON スキーマと監査手順を [3-7-core-config-data.md](3-7-core-config-data.md) §4 に移設し、`Core.Config`／`Core.Diagnostics` と同じ命名規約で管理します。
- ランタイムブリッジの Stage/Capability 契約を [3-8-core-runtime-capability.md](3-8-core-runtime-capability.md) §10 として正式化し、監査コードとの突合せを `Diagnostic.extensions["bridge"]` に定義しました。
- FFI と Unsafe ポインタ API を [3-9-core-async-ffi-unsafe.md](3-9-core-async-ffi-unsafe.md) §2–§3 へ編入し、`effect {memory}` と監査テンプレートを共通化しています。
- DSL プラグイン契約を [5-7-core-parse-plugin.md](5-7-core-parse-plugin.md) に集約し、Capability Stage と署名検証フローを Chapter 3/5 間で共有しました。
- CLI 診断出力を `SerializedDiagnostic` 基盤に統合し、`--format text` / `--format json` を単一オプションで選択できるよう再設計しました。CI 向けの `--format text --no-snippet` モードを追加し、差分検証は [reports/diagnostic-format-regression.md](../../reports/diagnostic-format-regression.md) で追跡しています。

## 付随ドキュメント

- ガイド (`../guides/`) は実装・運用上のベストプラクティスをまとめたハンドブック群です。
- ノート (`../notes/`) には設計検討メモ、ロードマップ、調査資料が保管されています。
- 計画書 (`../plans/bootstrap-roadmap/`) はブートストラップからセルフホスト移行までの実装計画と測定指標をまとめています。
- サンプル (`../examples/`) には Reml のコード例や運用スニペットが含まれます。

## 監査リソースと実装ベース

- Phase 2-8 の仕様監査成果物は `reports/spec-audit/`（`ch0/`〜`ch3/`, `diffs/`, `summary.md`）に集約し、Chapter ごとの CLI 出力・リンクチェック結果・`rust-gap` メモを保存します。保存ポリシーは [reports/spec-audit/README.md](../../reports/spec-audit/README.md) を参照してください。
- Rust Frontend (`compiler/frontend` の `poc_frontend`) が唯一のアクティブ実装です。`cargo test --manifest-path compiler/frontend/Cargo.toml` と `cargo run --bin poc_frontend -- --emit-*` が仕様検証時の標準コマンドであり、`docs/plans/rust-migration/overview.md` / `.../unified-porting-principles.md` に基準が整理されています。
- Chapter 1 のコード片は `examples/docs-examples/spec/1-1-syntax/` に `.reml` として切り出し、監査ログ (`reports/spec-audit/ch1/*.json`) と相互参照します。Rust Frontend がまだ受理できない構文は `*_rustcap.reml` / `rust-gap` 脚注で制限を明示します。

## 次の読み進め方

言語仕様の流れを把握したい場合は Chapter 1 → Chapter 2 → Chapter 3 の順で辿ると、構文定義から標準ライブラリへの橋渡しを自然に理解できます。プラグインやエコシステム領域に関心がある場合は Chapter 3 の Capability セクションを起点に Chapter 4・5 を参照し、ガイド/ノートで補足情報を確認してください。
