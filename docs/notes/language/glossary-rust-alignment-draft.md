# TODO: Rust 用語整合下書き

Rust 実装移行で導入する主要語彙の仮定義を整理する。`docs/spec/0-2-glossary.md`
へ追加する前提でレビューを募るための下書きであり、採用時は各エントリを仕様書へ
移設する。

> ✅ 2025-11-08: 所有権・借用関連語彙を `docs/spec/0-2-glossary.md#所有権とリソース管理`
> へ反映済み。以下は追加語彙を検討する際の補助メモとして維持する。

- **所有権 (Ownership)**: 値とそのメモリ資源に対して唯一の管理主体を割り当て、所有者が
  スコープを離れるタイミングで解放責務を果たす規約。Reml ではライフサイクル管理の基本
  単位として扱い、所有権移動は値のムーブと同義とする。参考: `docs/plans/rust-migration/unified-porting-principles.md`
  §3。
- **借用 (Borrow)**: 所有権を移さずに値へ一時的アクセスを許す操作。Reml の `effects`
  と Capability 監査では、借用中に所有者が破棄されないことを検証対象とする。
- **借用検査 (Borrow Checker)**: 借用ルールの静的検証機構。Rust ではコンパイラが同名の
  フェーズを持ち、Reml では `effects`/Capability 検証の一部として観測されるため
  「借用検査」と訳出する。
- **ライフタイム (Lifetime)**: 値や参照が有効な区間を表す注釈。Rust の `'a` 記法を説明する際は
  「ライフタイム `'a`」のように原語を添える。
- **共有借用 (Shared Borrow)**: 読み取り専用で複数存在できる借用。Reml では Capability
  の `readonly` 相当として扱い、競合のない観測を許可する。
- **可変借用 (Mutable Borrow)**: 同時に 1 つだけ許される書き込み可能な借用。Reml の
  可変 Capability (`mut`) と対応付ける。
- **ムーブ (Move Semantics)**: 所有権を別の束縛へ移す操作。ムーブ後の元の束縛は未初期化
  状態とみなし、再利用する場合は再初期化が必要である。
- **非安全ブロック (unsafe block)**: Rust の `unsafe { ... }` 構文。Reml 仕様では
  非安全操作を Capability 監査で追跡するため、「非安全ブロック (`unsafe`)」と表記する。

> **レビュー手順メモ**: 各エントリは `docs/plans/rust-migration/appendix/glossary-alignment.md`
> の参照列からリンクされる。仕様書更新時は本ノートをクローズし、正式定義を
> `docs/spec/0-2-glossary.md` へ移動させる。
