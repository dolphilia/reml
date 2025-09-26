# Reml 設計インスピレーション調査

## 1. 言語コアに影響を与えたと推測される技術

- **Haskell / OCaml / F# 系**（`1-2-types-Inference.md`, `0-1-overview.md`）
  - Hindley–Milner 型推論とランク1多相、代数的データ型、パターンマッチ、パイプ演算子など関数型言語の中核機能を採用。
  - 型クラス風トレイトや `match` 構文、宣言的な関数合成スタイルが強く反映されている。
- **Rust**（`0-1-overview.md`, `1-1-syntax.md`, `notes/core-library-scope.md`）
  - `Result`/`Option` と `?` 演算子、`trait`/`impl`、安全境界としての `unsafe` ブロック、`@cfg` 条件付きコンパイル、所有権を意識したゼロコスト抽象志向などを共有。
  - DSL や標準ライブラリの設計で Capability Registry や CLI 統合を重視する姿勢も Rust エコシステムと共通。
- **Koka / Eff 系効果システム研究**（`1-3-effects-safety.md`）
  - 効果タグの細分化と将来的な行多相ベース効果型の導入検討が明記されており、研究言語の影響が示唆される。

## 2. パーサーとエラーモデルの参照元

- **Parsec / FParsec / Megaparsec / FastParse / PEG 系**（`2-1-parser-type.md`, `guides/early-design-concepts.md`, `2-6-execution-strategy.md`）
  - `consumed/committed` 管理や `cut`、`attempt`、Packrat メモ化、左再帰サポートなど、既存パーサーコンビネーター実装の知見を統合。
  - `precedence` ビルダーやエラー期待集合の整備など、複数ライブラリの長所取り込みを明言。

## 3. エコシステム・ツールチェーンの影響源

- **Cargo / npm / pip / Go modules**（`reml-ecosystem-analysis.md`, `notes/cross-compilation-spec-intro.md`）
  - 統合CLI、中央レジストリ、ターゲットプロファイル管理、競争的エコシステム設計などを比較研究し Reml CLI/レジストリ設計指針を抽出。
  - クロスコンパイルやツールチェーン分離は Rustup/Clang/Go の運用を参照。

## 4. コンパイラ実装技術

- **LLVM**（`guides/llvm-integration-notes.md`, `2-6-execution-strategy.md`）
  - IR 生成フロー、データレイアウト、ターゲットトリプル整合など LLVM ベースのバックエンドを前提にした設計。
- **OCaml**（`guides/llvm-integration-notes.md`）
  - ブートストラップ段階で OCaml + Menhir を利用する計画が示され、型推論やコンパイラ実装経験を活用。

## 5. 標準ライブラリとDSL設計への示唆

- **Rust / F# / OCaml 標準ライブラリ**（`notes/core-library-scope.md`）
  - コレクション、テキスト、診断 API などでこれらのエコシステムを比較参照し、Reml の API 範囲と優先度を決定。
- **DSL ファースト思想**（`0-2-project-purpose.md`, `guides/early-design-concepts.md`）
  - パーサーコンビネーターを言語コアに内蔵し、DSL 構築と相互運用を最優先する設計哲学。既存 DSL ツール（ANTLR, PEG.js 等）との性能比較を意識。

## 6. まとめ

Reml は関数型言語と Rust の安全志向を組み合わせ、パーサーコンビネーター界隈（Parsec 系、PEG 技術）の実務知見を体系化した設計である。ツールチェーンやエコシステム面では Cargo や npm など既存成功例を分析し、LLVM・OCaml を足掛かりに実装計画を立てている。DSL ファースト方針により、他言語の弱点だったエラー品質と国際化サポートを初期から統合する点が特徴的である。
