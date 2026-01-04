# Reml 高速化・最適化に関する調査と提案 (2025-12-21)

## 1. はじめに

Reml プロジェクトは「実用に耐える性能 (O(n) パース、低メモリフットプリント)」を最優先の価値観として掲げています。
本調査では、Rust への移行 (Phase 3以降) を前提とし、現代的なハードウェアリソースを最大限に活用して Reml をさらに高速化するための技術的選択肢を検討します。

**主な焦点**:
- **最優先**: パーサおよびコンパイラ基盤の高速化 (O(n) 達成、省メモリ)
- **次点**: マルチコア CPU の活用 (Parallelism)
- **将来検討**: GPU コンピュート、DSL の JIT 実行

本提案は、`docs/spec/0-1-project-purpose.md` の「実用に耐える性能」と「安全性」を両立させるため、すべての最適化を `RunConfig` および Capabilities で制御可能にすることを前提とします。

## 2. 最適化ガバナンスと安全性 (Safety & Governance)

Reml では、性能向上が安全性を損なわないよう、以下のルールを適用します。

*   **Opt-in 原則**: SIMD, JIT, GPU, 並列化などのハードウェア依存最適化は `RunConfig` (または `@cfg`) による明示的な許可を必要とする。
*   **Fallback 必須**: 最適化パスが利用できない環境 (WASM, 古い CPU) のために、必ずポータブルな実装へのフォールバックを用意する。
*   **監査ログ**: 最適化の適用状況 (e.g., "JIT Enabled", "GPU Backend Active") を監査ログ (`audit.log`) に記録し、再現性を担保する。

## 3. CPU/コンパイラ基盤の最適化 (Core Optimizations)

Reml の「O(n) パース」目標を達成するための、アルゴリズムおよびデータ構造レベルの最適化です。

### 3.1 パーサ高速化戦略

*   **Zero-copy Input (実験的)**:
    *   *概念*: 入力ソースコード (`&[u8]`) をコピーせず、スライス参照だけで AST を構築する。
    *   *課題*: Unicode 正規化が必要な箇所との整合性。`Cow<str>` を活用し、変更が必要な場合のみアロケーションを行う戦略を検討。
    *   *参照*: `docs/notes/parser/core-parse-improvement-survey.md`
*   **Packrat キャッシュ制御**:
    *   *提案*: メモ化の粒度を調整可能にする。すべてのルールをキャッシュするとメモリ圧迫するため、`@memo` 属性が付いたルールまたは再帰頻度が高いルールのみ動的にキャッシュする。
    *   *Cut 演算子の活用*: バックトラックを明示的に禁止する `cut` 演算子の導入により、不要なパース試行を削減する。

### 3.2 Core IR 最適化

`docs/plans/bootstrap-roadmap/1-3-core-ir-min-optimization.md` で計画されている最適化を Phase 3 までに確実に実装します。

*   **DCE / 定数畳み込み**: コンパイル時に不要なコード・分岐を削除し、ランタイム負荷を下げる。
*   **Inline Expansion**: 小規模な関数や DSL のヘルパー関数をインライン展開し、関数呼び出しオーバーヘッドを削減。

## 4. データ処理と SIMD (Data & SIMD)

### 2.1 SIMD (Single Instruction, Multiple Data) の活用

Reml がターゲットとする x86_64, ARM64 は強力な SIMD 命令セットを持っています。
Rust 実装において以下の戦略が考えられます。

*   **Auto-vectorization**: コンパイラ (LLVM) が自動的にベクトル化しやすいコードを書く。
    *   *提案*: `Core.Iter` や `Core.Collection` の実装において、内部イテレータ (`fold`, `for_each`) を優先し、スライスへの直接アクセス (`windows`, `chunks`) を積極的に利用する。
*   **Portable SIMD**: Rust の `std::simd` (Nightly) または `simba` クレートの利用。
    *   *提案*: 文字列処理 (`Core.Text`) や数値計算ライブラリ (`Core.Numeric`) のホットパスにおいて、SIMD を明示的に利用するバックエンドを用意する。
    *   *課題*: Nightly 依存のリスク。安定版では `bytemuck` 等を用いた安全なキャストと `cfg` マクロによる条件付きコンパイルで対応する。

### 2.2 データ指向設計 (Data-Oriented Design)

DSL のデータ処理において、キャッシュ効率を高めるメモリレイアウトを標準ライブラリでサポートします。

*   **SoA (Structure of Arrays)**:
    *   *現状*: Reml の `Record` は通常 AoS (Array of Structures) になる。
    *   *提案*: `Core.Data.Frame` のような構造体を提供し、内部的に SoA レイアウトでデータを保持・処理するコンテナを導入する。これにより、特定フィールドのみの集計などが爆発的に高速化される。

## 5. 並列処理 (Parallel Processing)

### 3.1 データ並列 (Data Parallelism)

DSL は大量のデータを処理するシナリオが多いため、ユーザーが意識せずに並列化の恩恵を受けられるようにします。

*   **Rayon の統合**:
    *   *提案*: `Core.Iter.Parallel` モジュールを提供し、Rust の `rayon` クレートをラップする。
    *   *API*: `iter.par_map(...)` のような API を提供。
    *   *Capability*: `core.runtime.parallel` Capability を要求することで、リソース消費を制御・監査可能にする。

### 3.2 パイプライン並列

Reml のコンパイラ自体や、ストリーム処理 DSL の高速化。

*   **Async/Await ランタイム**:
    *   すでに `Core.Async` が計画されているが、これを計算集約タスクにも応用する。Work-stealing ランタイム (Tokio/Smol) 上で、IO 待ちと計算タスクを効率的にミックスする。

### 3.3 並列コンパイル (Parallel Query System)

セルフホストコンパイラの高速化。

*   **Query System**: Rust 製ビルドツールや LSP で採用される `salsa` のようなクエリベースのアーキテクチャを採用する。
*   *効果*: 依存関係のないモジュールの並列パース・型チェックが可能になり、LSP の応答速度が向上する。

## 6. GPU アクセラレーション (Experimental)

特定のドメイン (画像処理、数値シミュレーション、大規模データ解析) 向け DSL において、GPU は CPU の数千倍の性能を発揮する可能性があります。

### 4.1 WebGPU (wgpu) の採用

*   **技術選定**: `wgpu` は Rust エコシステムで標準的な、クロスプラットフォームかつ安全性が高い Graphics/Compute API。
    *   Vulkan, Metal, DX12, WebGL/WebGPU に変換されるため、Reml のポータビリティを損なわない。
*   **提案**: `Core.Gpu` ライブラリの新設。
    *   コンピュートシェーダー (WGSL) を Reml 文字列として記述、あるいは Reml のサブセットから WGSL へトランスパイルする。
    *   *Capability*: `gpu.compute` Capability で保護。監査ログにシェーダーハッシュや実行時間を記録。

### 4.2 Use Cases
*   **画像処理 DSL**: ピクセル単位の並列操作。
*   **AI/Tensor DSL**: 行列演算のオフロード。

## 7. JIT コンパイルと動的実行 (Experimental)

インタプリタ実行よりも高速で、AOT (Ahead-of-Time) コンパイルよりも手軽な実行手段。

### 5.1 Cranelift

*   **特徴**: Rust 製のコードジェネレータ。LLVM よりコンパイルが非常に高速 (10倍以上) で、生成コードの品質も悪くない。
*   **提案**: `reml run` コマンドの開発用バックエンド、または DSL の JIT 実行エンジンとして採用。
    *   ユーザー定義関数を瞬時にネイティブコード化して実行できる。

### 5.2 WASM Runtime (Wasmtime)

*   **特徴**: 安全なサンドボックス実行。
*   **提案**: Reml を WASM にコンパイルし、`Wasmtime` で実行するモード。
    *   *メリット*: メモリ安全性が保証される。プラグインシステム (`Core.Plugin`) の基盤として最適。FFI の境界コストはあるが、セキュリティリスクを隔離できる。
    *   *Remlとの相性*: Capability System と WASM の Capability-based security (WASI) は概念的に非常に親和性が高い。

## 8. Reml への導入ロードマップ案

### Phase 2-3: 基盤最適化 (Priority)
1.  **Core IR Optimizations**: DCE, Linlining などの基本パスを確立 (`1-3-core-ir-min-optimization.md`).
2.  **Parser Tuning**: Packrat キャッシュ戦略の調整とベンチマーク (`lexer-performance-study.md`).

### Phase 4: 実証実験
1.  **Parallel Iterators**: `examples/practical` の重い処理で `rayon` を試行導入。
2.  **JIT Prototype**: `reml run` の実験的バックエンドとして Cranelift/WASM を検証 (`a-jit.md`).

### Phase 5-6: エコシステム拡張
1.  **Query System**: コンパイラ内部構造への `salsa` 導入検討。
2.  **Core.Gpu**: 実験的パッケージとして公開し、特定ドメインでのみ許可。

## 9. 結論と推奨

Reml の「安全性」「実用性」を維持しつつ高速化するには、以下の順序で取り組むべきです。

1.  **Parser/IR (Phase 2-3)**: 最優先。ゼロコピーや IR 最適化により、基本性能（パーススループット）を底上げする。
2.  **Parallel (Phase 4)**: `Rayon` によるデータ並列化。コスト対効果が高く比較的安全。
3.  **JIT/WASM (Phase 4+)**: プラグインと動的実行の要。セキュリティモデルとの整合を慎重に設計する。
4.  **GPU (Phase 6)**: 特定用途向け拡張 (`Core.Gpu`) として切り出し、安全なサンドボックス内でのみ許可する。

---
*文責: Antigravity Assistant*
