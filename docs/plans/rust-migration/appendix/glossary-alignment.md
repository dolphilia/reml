# Appendix Rust↔Reml 用語整合

本付録は Rust 実装移行時に発生する用語の差異を整理し、Reml 仕様（`docs/spec/0-2-glossary.md`）で定義された語彙と整合させるための指針をまとめる。Phase P0 のレビュー時点で必要となる語を初期登録し、以降のフェーズでは本表を拡張しながら運用する。

## A.1 対応表

| Rust 側用語 | Reml 仕様上の用語 | 説明 / 使い分け | 参照 |
| --- | --- | --- | --- |
| Ownership | 所有権 | Rust の所有権規則を Reml のリソース管理規約に対応させる。Reml 仕様ではライフサイクル管理を「所有権」と呼称する。 | `docs/spec/0-2-glossary.md` 「所有権」 |
| Borrow / Borrow Checker | 借用 / 借用検査 | Rust コンパイラの参照管理。Reml では `effects` と Capability 監査で類似制約を扱うため、「借用（参照一時貸与）」と表記し、必要なら括弧に原語を併記する。 | `docs/spec/1-3-effects-safety.md` §B |
| Lifetime | ライフタイム | Reml 仕様ではリソース存続期間を「ライフタイム」と呼ぶ。Rust の `'a` 記法を解説する際は「ライフタイム（'a）」と併記する。 | `docs/spec/0-2-glossary.md` 「ライフタイム」 |
| Result<T, E> | `Result<T, E>` 型 | Reml 仕様でも `Result` が標準化されているため、Rust と同名で扱う。Rust 固有の `?` 演算子は「早期戻り (`?` 演算子)」と説明する。 | `docs/spec/3-0-core-library-overview.md` §Result |
| Option<T> | `Option<T>` 型 | Null 代替の選択型。Reml 仕様と意味が同一のため名称をそのまま使用する。 | 同上 §Option |
| Unsafe | 非安全ブロック | Rust の `unsafe` は Reml 仕様の「非安全ブロック」と対応付ける。説明時は `unsafe` キーワードをコード表記で併記する。 | `docs/spec/3-9-core-async-ffi-unsafe.md` |
| Crate | クレート（Rust パッケージ） | Reml ではパッケージを一般に「パッケージ」と呼ぶが、Rust 固有の単位を扱う際は「クレート（Rust パッケージ）」と表記する。 | `docs/spec/5-1-package-lifecycle.md` |
| Cargo | Cargo（ビルドツール） | Rust の公式ビルド/パッケージ管理ツール。Reml の `reml package` 等と区別するため、原語のまま使用し脚注で説明する。 | `docs/guides/tooling-overview.md`（更新予定） |
| Target Triple | ターゲット Triple | LLVM/Rust で使用するターゲット識別子。Reml 仕様では `TargetTriple` と表記しているため、説明時は「ターゲット Triple (`x86_64-pc-windows-msvc`)」のように併記する。 | `docs/plans/bootstrap-roadmap/windows-llvm-build-investigation.md` |
| Feature Flag | フィーチャフラグ | Rust の `#[cfg(feature = ...)]` と Reml の Capability フラグを区別するため、「フィーチャフラグ（Rust）」と「Capability フラグ」を明示的に使い分ける。 | `docs/spec/3-8-core-runtime-capability.md` |
| Module | モジュール | Reml 仕様では `module` を「モジュール」と訳出済み。Rust の `mod` と対応づけて説明するが、名前空間構造が異なる場合は「Rust モジュール」と記す。 | `docs/spec/0-2-glossary.md` 「モジュール」 |

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
