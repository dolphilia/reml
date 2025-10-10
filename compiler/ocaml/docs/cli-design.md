# CLI アーキテクチャ設計判断の記録

**Phase**: 1-6 開発者体験整備
**作成日**: 2025-10-10
**対象**: Week 14-16 (CLIアーキテクチャ設計)

## 目的

このドキュメントは、Phase 1-6 の CLI アーキテクチャ設計において行った設計判断とその理由を記録します。将来の保守や拡張時に判断の背景を理解できるようにすることが目的です。

---

## 設計判断

### D1: CLI コードを `tooling/cli/` に配置

**日付**: 2025-10-10

**決定内容**:
CLI 関連のコードを `compiler/ocaml/src/` ではなく、`tooling/cli/` に配置する。

**理由**:
1. **関心の分離**: コンパイラコアとツール層を明確に分離
2. **Phase 2 への移行**: セルフホスト時に CLI だけを Reml で書き直すことが容易
3. **再利用性**: 他のツール（LSP、フォーマッター等）と共通のインフラを共有可能

**代替案と却下理由**:
- **案A**: `compiler/ocaml/src/cli/` に配置
  - 却下理由: コンパイラコアとの結合度が高くなり、Phase 2 での移行が困難
- **案B**: `tooling/` 直下に配置
  - 却下理由: 複数のツールが混在し、構造が不明瞭になる

**影響**:
- ビルドシステム（dune）で `tooling/cli/` を別ライブラリとして扱う必要がある
- 既存の `main.ml` からの移行作業が必要

**参考**:
- [ARCHITECTURE.md](../../../tooling/cli/ARCHITECTURE.md)
- [0-1-project-purpose.md](../../../docs/spec/0-1-project-purpose.md) §3.2

---

### D2: オプション解析を独立したモジュールに分離

**日付**: 2025-10-10

**決定内容**:
コマンドラインオプションの定義と解析を `options.ml` として独立したモジュールに分離する。

**理由**:
1. **テスタビリティ**: オプション解析を単独でテストできる
2. **保守性**: オプション追加時の変更箇所が明確
3. **型安全性**: オプション設定を構造体として扱い、文字列アクセスを排除

**設計詳細**:
```ocaml
type options = {
  input_file: string;
  emit_ast: bool;
  emit_tast: bool;
  (* ... *)
}
```

**代替案と却下理由**:
- **案A**: `main.ml` に直接実装
  - 却下理由: テストが困難、main.ml が肥大化
- **案B**: `Cmdliner` ライブラリを使用
  - 却下理由: 依存関係増加、Phase 1 では過剰な機能

**Phase 2 での見直し**:
- Phase 2 では `Cmdliner` の導入を検討（サブコマンド対応のため）

**参考**:
- [OPTIONS.md](../../../tooling/cli/OPTIONS.md)

---

### D3: 診断出力を `text` と `json` の2形式に対応

**日付**: 2025-10-10

**決定内容**:
診断メッセージの出力形式を `text`（デフォルト）と `json` の2つに対応する。

**理由**:
1. **人間可読性**: `text` 形式で開発者が直感的に理解できる
2. **機械判読性**: `json` 形式で CI/CD ツールが処理できる
3. **LSP 互換**: `json` 形式を LSP プロトコルに近づけることで Phase 2 への移行を容易化

**設計詳細**:
- `--format=text` (デフォルト): カラーコード対応、ソースコードスニペット表示
- `--format=json`: 仕様書 3-6 準拠の JSON スキーマ

**代替案と却下理由**:
- **案A**: `text` 形式のみ
  - 却下理由: CI/CD 連携が困難
- **案B**: `xml` 形式も追加
  - 却下理由: Phase 1 では需要が不明、実装コスト増

**Phase 2 での見直し**:
- `html`, `markdown` 形式の追加を検討

**参考**:
- [3-6-core-diagnostics-audit.md](../../../docs/spec/3-6-core-diagnostics-audit.md)
- [ARCHITECTURE.md](../../../tooling/cli/ARCHITECTURE.md) §4.4

---

### D4: カラー出力を `auto`, `always`, `never` の3モードで制御

**日付**: 2025-10-10

**決定内容**:
カラー出力を `--color=auto|always|never` で制御し、環境変数 `NO_COLOR` にも対応する。

**理由**:
1. **標準準拠**: `NO_COLOR` 環境変数は事実上の標準
2. **UX 向上**: TTY 判定を `auto` で自動化し、ユーザーの操作を減らす
3. **CI/CD 対応**: パイプ時には自動的にカラーを無効化

**設計詳細**:
- `auto` (デフォルト): `Unix.isatty` で TTY 判定
- `always`: 常にカラーコードを出力
- `never`: カラーコードを出力しない
- 環境変数 `NO_COLOR` が設定されている場合は `never` として動作

**代替案と却下理由**:
- **案A**: カラー出力を常に有効
  - 却下理由: パイプ時にエスケープシーケンスが混入
- **案B**: 環境変数のみで制御
  - 却下理由: CLI オプションでの制御がより直感的

**実装注意点**:
- `Unix.isatty` は stderr の FD で判定（stdout ではない）

**参考**:
- [no-color.org](https://no-color.org/)
- [OPTIONS.md](../../../tooling/cli/OPTIONS.md) §診断オプション

---

### D5: トレース機能を `--trace` オプションで提供

**日付**: 2025-10-10

**決定内容**:
コンパイルフェーズのトレース（時間計測、メモリ使用量）を `--trace` オプションで提供する。

**理由**:
1. **パフォーマンス分析**: どのフェーズが遅いかを可視化
2. **デバッグ支援**: 問題発生時の診断情報を提供
3. **メトリクス収集**: 0-3-audit-and-metrics.md への記録に活用

**設計詳細**:
- 各フェーズの開始・終了時に `Unix.gettimeofday` で時間計測
- `Gc.stat` でメモリ使用量を取得
- stderr に出力（stdout は AST 等の出力に使用）

**代替案と却下理由**:
- **案A**: 常にトレースを有効化
  - 却下理由: 出力が煩雑になる、性能オーバーヘッド
- **案B**: 環境変数のみで制御
  - 却下理由: CLI オプションの方が発見しやすい

**Phase 2 での見直し**:
- トレース情報を JSON 形式で出力するオプションの追加

**参考**:
- [1-6-developer-experience.md](../../../docs/plans/bootstrap-roadmap/1-6-developer-experience.md) §4
- [0-3-audit-and-metrics.md](../../../docs/plans/bootstrap-roadmap/0-3-audit-and-metrics.md)

---

### D6: 統計情報を `--stats` オプションで提供

**日付**: 2025-10-10

**決定内容**:
コンパイル統計（トークン数、AST ノード数、unify 呼び出し回数等）を `--stats` オプションで提供する。

**理由**:
1. **性能評価**: 仕様書の性能目標（10MB ファイルの線形処理）を検証
2. **開発者情報**: コンパイラ内部の挙動を可視化
3. **回帰検出**: CI で統計を記録し、性能劣化を検出

**設計詳細**:
```
[STATS] Tokens parsed: 42
[STATS] AST nodes: 18
[STATS] Unify calls: 35
[STATS] Optimization passes: 3
[STATS] LLVM instructions: 127
```

**代替案と却下理由**:
- **案A**: `--trace` に統合
  - 却下理由: 関心の分離（時間計測と統計は別の関心事）
- **案B**: JSON 出力のみ
  - 却下理由: 人間可読な出力も必要

**Phase 2 での見直し**:
- `--metrics-output=<file>` で JSON 形式の詳細メトリクスを出力

**参考**:
- [0-1-project-purpose.md](../../../docs/spec/0-1-project-purpose.md) §1.1 性能基準

---

### D7: パイプラインオーケストレーションを `pipeline.ml` に集約

**日付**: 2025-10-10

**決定内容**:
コンパイルフェーズの管理（Parser → Typer → Core IR → Codegen）を `pipeline.ml` に集約する。

**理由**:
1. **関心の分離**: main.ml はオプション解析のみに専念
2. **再利用性**: パイプライン処理を LSP やテストから再利用可能
3. **拡張性**: Phase 2 でインクリメンタルコンパイルを追加しやすい

**設計詳細**:
```ocaml
type context = {
  options: Options.options;
  source: string;
  trace_enabled: bool;
  stats_enabled: bool;
}

val run : context -> unit pipeline_result
```

**代替案と却下理由**:
- **案A**: main.ml に直接実装
  - 却下理由: main.ml が肥大化、再利用困難
- **案B**: 各フェーズを個別の関数として定義
  - 却下理由: フェーズ間の依存関係が不明瞭

**実装注意点**:
- エラー時は早期リターン（`Result` 型を活用）
- トレース・統計収集は `context` の設定に基づいて条件分岐

**参考**:
- [ARCHITECTURE.md](../../../tooling/cli/ARCHITECTURE.md) §4.2

---

### D8: 既存 `main.ml` の段階的リファクタリング

**日付**: 2025-10-10

**決定内容**:
既存の `compiler/ocaml/src/main.ml` を一度にリファクタリングするのではなく、段階的に機能を分離する。

**理由**:
1. **リスク低減**: 大規模な変更を一度に行うとバグ混入のリスクが高い
2. **テスト維持**: 既存テストを常に通過させながら進める
3. **レビュー容易性**: 小さな変更ごとにレビューできる

**段階的リファクタリング計画**:

#### Phase 1（Week 14前半）: オプション分離
1. `options.ml` を作成し、オプション定義を移行
2. `main.ml` は `Options.parse_args` を呼び出すのみ

#### Phase 2（Week 14後半）: 診断フォーマッター分離
1. `diagnostic_formatter.ml` を作成
2. `main.ml` の診断出力を `diagnostic_formatter` へ移行

#### Phase 3（Week 15前半）: パイプライン分離
1. `pipeline.ml` を作成
2. `main.ml` のコンパイルフロー を `Pipeline.run` へ移行

#### Phase 4（Week 15後半）: トレース・統計分離
1. `trace.ml`, `stats.ml` を作成
2. `pipeline.ml` からトレース・統計処理を移行

**各段階での検証**:
- 既存テスト（143件）が全て成功すること
- `--emit-ast`, `--emit-tast`, `--emit-ir` の動作が変わらないこと

**代替案と却下理由**:
- **案A**: 一度に全てリファクタリング
  - 却下理由: リスクが高い、レビュー困難
- **案B**: Phase 2 まで既存 main.ml を維持
  - 却下理由: Phase 1-6 の目標（DX 整備）を達成できない

**参考**:
- [1-5-to-1-6-handover.md](../../../docs/plans/bootstrap-roadmap/1-5-to-1-6-handover.md) §3.1

---

### D9: Phase 1 では標準入力（`-`）をサポートしない

**日付**: 2025-10-10

**決定内容**:
Phase 1-6 では標準入力からのソースコード読み込み（`remlc -`）をサポートしない。

**理由**:
1. **優先度**: Phase 1-6 の主目的は診断出力強化とトレース機能
2. **実装コスト**: 標準入力対応には追加の複雑性（ファイル名の扱い、エラー位置の報告等）
3. **需要**: Phase 1 では CLI からのファイル指定が主な使用方法

**Phase 2 での対応**:
- `remlc -` をサポート
- `--stdin-filename` オプションで仮想ファイル名を指定可能にする

**代替案と却下理由**:
- **案A**: Phase 1 で実装
  - 却下理由: 実装コストが高く、Phase 1-6 のスケジュールに影響

**参考**:
- [OPTIONS.md](../../../tooling/cli/OPTIONS.md) §入力オプション

---

### D10: ヘルプメッセージをセクション別に整理

**日付**: 2025-10-10

**決定内容**:
`--help` の出力をセクション別（入力、出力、診断、デバッグ、コンパイル）に整理する。

**理由**:
1. **可読性**: オプションが多い場合でも見やすい
2. **学習曲線**: 初心者は基本オプションから順に学べる
3. **標準準拠**: `clang`, `rustc` 等の主要コンパイラと同様の形式

**設計詳細**:
```
USAGE:
  remlc [OPTIONS] <file.reml>

INPUT:
  <file.reml>          Input Reml source file

OUTPUT:
  --emit-ast           Emit AST to stdout
  --emit-ir            Emit LLVM IR to output directory
  ...

DIAGNOSTICS:
  --format <format>    Output format: text|json
  --color <mode>       Color mode: auto|always|never
  ...
```

**代替案と却下理由**:
- **案A**: アルファベット順に列挙
  - 却下理由: 関連するオプションが分散し、理解しにくい
- **案B**: 詳細なマニュアルページ（man page）のみ
  - 却下理由: CLI での簡易ヘルプも必要

**Phase 2 での見直し**:
- サブコマンド追加時に階層的なヘルプ構造を導入

**参考**:
- [OPTIONS.md](../../../tooling/cli/OPTIONS.md) §その他
- [0-1-project-purpose.md](../../../docs/spec/0-1-project-purpose.md) §2.1 書きやすさ

---

## 却下した設計案

### R1: `Cmdliner` ライブラリの使用

**検討日**: 2025-10-10

**提案内容**:
OCaml の `Cmdliner` ライブラリを使用してオプション解析を実装する。

**却下理由**:
1. **依存関係**: 新しい外部ライブラリへの依存を追加したくない
2. **Phase 1 スコープ**: Phase 1 では単純なオプション解析で十分
3. **学習コスト**: `Cmdliner` の DSL を学ぶコストが見合わない

**Phase 2 での再検討**:
サブコマンド（`reml build`, `reml test` 等）を追加する際に再検討する。

---

### R2: 設定ファイル（`reml.toml`）の Phase 1 実装

**検討日**: 2025-10-10

**提案内容**:
Phase 1-6 で `reml.toml` による設定ファイル対応を実装する。

**却下理由**:
1. **優先度**: Phase 1-6 の主目的は診断出力強化
2. **複雑性**: TOML パーサーの統合、CLI オプションとのマージ処理が複雑
3. **需要**: Phase 1 では単一ファイルのコンパイルが主用途

**Phase 2 での対応**:
- `reml.toml` をサポート
- プロファイル（`[profile.dev]`, `[profile.release]`）も実装

---

### R3: LSP サーバの Phase 1 実装

**検討日**: 2025-10-10

**提案内容**:
Phase 1-6 で Language Server Protocol サーバを実装する。

**却下理由**:
1. **スコープ**: Phase 1-6 の計画書に明示的に除外されている
2. **複雑性**: LSP プロトコル実装は大規模な作業
3. **優先度**: CLI の基盤整備が先決

**Phase 2 での対応**:
- LSP サーバ実装を開始
- CLI の診断出力インフラを再利用

---

## 今後の検討事項

### F1: パフォーマンステストの自動化

**検討予定**: Phase 1-6 Week 16

**内容**:
`--trace` と `--stats` の出力を CI で自動的に記録し、性能回帰を検出する仕組みを整備する。

**課題**:
- 実行環境による性能差の吸収方法
- 基準値の設定方法

---

### F2: メトリクス出力の JSON 形式対応

**検討予定**: Phase 2

**内容**:
`--metrics-output=<file>` オプションで詳細なメトリクスを JSON 形式で出力する。

**利点**:
- CI/CD ツールでの自動処理が容易
- メトリクスの可視化ツールとの連携

---

### F3: 診断メッセージの多言語対応

**検討予定**: Phase 3

**内容**:
診断メッセージを英語・日本語で切り替え可能にする。

**課題**:
- メッセージカタログの管理方法
- 環境変数 `LANG` の扱い

---

## 参考資料

- [CLI アーキテクチャ](../../../tooling/cli/ARCHITECTURE.md)
- [オプションリファレンス](../../../tooling/cli/OPTIONS.md)
- [Phase 1-6 計画書](../../../docs/plans/bootstrap-roadmap/1-6-developer-experience.md)
- [プロジェクト目的](../../../docs/spec/0-1-project-purpose.md)
- [診断仕様](../../../docs/spec/3-6-core-diagnostics-audit.md)

---

**作成者**: Claude Code
**最終更新**: 2025-10-10
