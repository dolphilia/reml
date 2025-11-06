# Appendix Rust↔Reml 用語整合

本付録は Rust 実装移行時に発生する用語の差異を整理し、Reml 仕様（`docs/spec/0-2-glossary.md`）で定義された語彙と整合させるための指針をまとめる。Phase P0 のレビュー時点で必要となる語を初期登録し、以降のフェーズでは本表を拡張しながら運用する。

## A.1 対応表

| Rust 側用語 | Reml 仕様上の用語 | 説明 / 使い分け | 参照 |
| --- | --- | --- | --- |
| Ownership | 所有権 | Rust の所有権規則を Reml のリソース管理規約に対応させる。Reml ではライフサイクル管理の基本単位として扱い、ムーブは所有権移動と定義する。 | `docs/spec/0-2-glossary.md#所有権とリソース管理` |
| Borrow / Borrow Checker | 借用 / 借用検査 | Rust コンパイラの参照管理。Reml では `effects` と Capability 監査で類似制約を扱うため、「借用（参照一時貸与）」および「借用検査」と訳出する。 | `docs/spec/0-2-glossary.md#所有権とリソース管理` |
| Shared Borrow | 共有借用 | 読み取り専用で複数並行できる借用。Reml では `readonly` Capability に対応させ、競合しない観測を保証する。 | `docs/spec/0-2-glossary.md#所有権とリソース管理` |
| Mutable Borrow | 可変借用 | 書き込み可能で同時に 1 つのみ許される借用。Reml の `mut` Capability と整合を取る。 | `docs/spec/0-2-glossary.md#所有権とリソース管理` |
| Lifetime | ライフタイム | リソース存続期間を示す注釈。Rust の `'a` 記法は「ライフタイム `'a`」と原語併記で説明する。 | `docs/spec/0-2-glossary.md#所有権とリソース管理` |
| Move | 所有権移動 (Move) | 所有権を別の束縛へ移動させる操作。移動元は未初期化扱いとなる点を Reml のリソース管理規約で明記する。 | `docs/spec/0-2-glossary.md#所有権とリソース管理` |
| Result<T, E> | `Result<T, E>` 型 | Reml 仕様でも `Result` が標準化されているため、Rust と同名で扱う。Rust 固有の `?` 演算子は「早期戻り (`?` 演算子)」と説明する。 | `docs/spec/3-0-core-library-overview.md` §Result |
| Option<T> | `Option<T>` 型 | Null 代替の選択型。Reml 仕様と意味が同一のため名称をそのまま使用する。 | 同上 §Option |
| Unsafe | 非安全ブロック (unsafe block) | Rust の `unsafe` は Reml 仕様の Capability 監査で追跡する非安全操作に相当するため、「非安全ブロック (`unsafe`)」と併記する。 | `docs/spec/0-2-glossary.md#所有権とリソース管理`, `docs/spec/3-9-core-async-ffi-unsafe.md#22-効果タグと-unsafe-境界` |
| Crate | クレート（Rust パッケージ） | Reml ではパッケージを一般に「パッケージ」と呼ぶが、Rust 固有の単位を扱う際は「クレート（Rust パッケージ）」と表記する。 | `docs/spec/5-1-package-lifecycle.md` |
| Cargo | Cargo（ビルドツール） | Rust の公式ビルド/パッケージ管理ツール。Reml の `reml package` 等と区別するため、原語のまま使用し脚注で説明する。 | `docs/guides/tooling-overview.md`（更新予定） |
| Target Triple | ターゲット Triple | LLVM/Rust で使用するターゲット識別子。Reml 仕様では `TargetTriple` と表記しているため、説明時は「ターゲット Triple (`x86_64-pc-windows-msvc`)」のように併記する。 | `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` |
| Feature Flag | フィーチャフラグ | Rust の `#[cfg(feature = ...)]` と Reml の Capability フラグを区別するため、「フィーチャフラグ（Rust）」と「Capability フラグ」を明示的に使い分ける。 | `docs/spec/3-8-core-runtime-capability.md` |
| Dual-write | 二重書き（dual-write）運用 | Rust 実装と OCaml 実装を並行実行して挙動差分を検証する運用。計画書では「二重書き（dual-write）」と併記し、差分ログは `reports/dual-write/` 配下へ保存する。 | `docs/plans/rust-migration/0-0-roadmap.md`, `docs/plans/rust-migration/1-0-front-end-transition.md` |
| Module | モジュール | Reml 仕様では `module` を「モジュール」と訳出済み。Rust の `mod` と対応づけて説明するが、名前空間構造が異なる場合は「Rust モジュール」と記す。 | `docs/spec/0-2-glossary.md` 「モジュール」 |
| Drop | 解放責務 (Drop) | Rust の `Drop` トレイトに相当する破棄処理。Reml では所有者のスコープ終了時に呼び出す解放責務として記述し、補助的に監査ログを参照する。 | `docs/spec/0-2-glossary.md#所有権とリソース管理`, `docs/spec/3-6-core-diagnostics-audit.md#diagnostic-bridge` |

## A.2 運用ルール
1. **原語併記の基準**  
   - 仕様語彙に既存の対訳がある場合は日本語を主とし、初出箇所で原語を括弧に入れる（例: 所有権（Ownership））。  
   - CI ログやコードコメントで原語しか出力できない場合は、計画書やガイドで補足説明を追加する。
2. **計画書更新時の手順**  
   - 新しい用語を導入した場合、本表へ追加し、`docs/spec/0-2-glossary.md` に未登録であれば別途更新案を提出する。  
   - 本表を更新した際は `docs/plans/rust-migration/README.md` から参照リンクを確認し、必要に応じて脚注を追記する。
3. **レビューの観点**  
   - 用語の揺れ（例: Borrow vs 借用）はレビュー時に指摘し、決定した表記を `docs/plans/rust-migration/0-0-roadmap.md` や関連計画書の脚注に反映する。  
   - `docs/spec/1-1-syntax.md` 等のコード例に影響がある場合は、コードスタイルガイド (`docs/spec/0-3-code-style-guide.md`) に従い再検証する。

## A.3 フォローアップタスク
- Phase P1 で AST/IR 仕様を Rust へ移植する際、`enum` と Reml のバリアント表記の対応表を追加する。
- Phase P2 で Runtime Capability を Rust へ導入する際、`unsafe` の用法と監査ログキー（`effect.stage.*`）の対応を追記する。
- Phase P3 で CI と監査メトリクスを統合する際、Rust 用語がダッシュボード上でどのように表示されるかを確認し、本表に補足する。

---

> **メモ**: 本表は計画書内に留め、仕様書・ガイドへ反映する場合は各文書の更新フロー（`docs/plans/bootstrap-roadmap/0-2-roadmap-structure.md`）に従って別途レビューを行うこと。
