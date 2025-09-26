# 5.2 Registry & Distribution（ドラフト）

> 目的：Reml パッケージレジストリおよび配布モデル（分散・中央ハイブリッド）を仕様化し、`reml publish` コマンド・依存解決の基盤を定義する。現状はアウトライン段階。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 草案（Draft） |
| 参照文書 | [reml-ecosystem-analysis.md](reml-ecosystem-analysis.md) §2, §4.2, §5.2 |
| 関連章 | 3-6, 3-7, 3-8, 4-1 |

## 1. レジストリアーキテクチャ

- 中央レジストリ（`registry.reml-lang.org` 仮称）と分散ミラー構成。
- REST/GraphQL API の方向性、認証方式（PAT/OIDC）。
- 耐障害性と CDN 配信戦略。

## 2. パッケージメタデータ

- `reml.toml` の `project` / `dependencies` / `dsl` 情報をどのように公開するか。
- 署名メタデータ、SBOM、セキュリティ勧告の付与。
- `DslCapabilityProfile`（3-8 §7）との同期。
- ターゲットメタデータの標準化：`targets` 配列を必須とし、各エントリに `profile_id`, `triple`, `runtime_revision`, `stdlib_version`, `capabilities`, `artifact`（URL）、`hash`, `format`（`remlpkg`, `staticlib` 等）を格納。

### 2.1 ターゲットメタデータ仕様

| フィールド | 型 | 説明 |
| --- | --- | --- |
| `profile_id` | `Str` | `TargetProfile.id`。`reml target list` で表示される ID と一致させる。 |
| `triple` | `Str` | LLVM triple (`x86_64-unknown-linux-gnu` 等)。`RunArtifactMetadata.llvm_triple` と突合。 |
| `runtime_revision` | `Str` | ランタイム互換リビジョン。`target.abi.mismatch` を防ぐため、CLI 側のランタイムと一致する必要がある。 |
| `stdlib_version` | `SemVer` | バンドル済み標準ライブラリのバージョン。`Core.Env.resolve_run_config_target` が検証。 |
| `capabilities` | `List<Str>` | `capability_name(TargetCapability::…)` で列挙した機能集合。`target.capability.unknown` 診断の根拠。 |
| `artifact` | `Url` | 対象ターゲット用バイナリアーティファクトまたは `remlpkg` への署名付き URL。 |
| `hash` | `Digest` | `RunArtifactMetadata.hash`。`sha256:` などアルゴリズムプレフィックス付き。 |
| `format` | `Str` | `"remlpkg"`, `"staticlib"`, `"cdylib"` 等。レジストリが MIME を決定する際に利用。 |
| `signature` | `Str` | 任意。Ed25519/Minisign 等の署名値。署名が無い場合は `null`。 |

- レジストリは `targets` 配列を `profile_id` でインデックス化し、`GET /packages/{name}/{version}` で JSON 応答を提供する。CLI は `profile_id` に基づいて適切なアーティファクトを選択し、`hash`/`signature` 検証を実行する。
- パッケージがターゲットを提供しない場合（ソースのみ）は `targets = []` を明示し、`requires_source_build = true` を添付して `reml add` が `--allow-source-build` を促すようにする。

## 3. 公開ワークフロー

1. `reml publish` 実行時のパイプライン（検証→ビルド→署名→アップロード→検証）。
2. 署名アルゴリズム（Ed25519 予定）と証明書チェーン。
3. アーティファクト再現性（deterministic tarball / lockfile）。
- ターゲット検証ステップ：アップロード前に CLI が `RunArtifactMetadata` を収集し、レジストリへ `targets[*]` を送信。レジストリは以下をチェックする。
  - `runtime_revision` と登録済みランタイム互換表（toolchain チャネル）を照合。失敗時は `target.abi.mismatch` を応答し、publish を拒否。
  - `capabilities` が既知か、`TargetCapability` または登録済みカスタム Capability と一致するか。未知の値は `400 target.capability.unknown` として返却。
  - `hash` が既存のアーティファクトと一致する場合は再アップロード禁止（リプレイ攻撃防止）。
  - `signature` が添付されている場合は公開鍵チェーンを検証し、`AuditEnvelope.metadata["signature"]` に記録する。
- 成功後、レジストリは `publish_receipt` を返し、`targets[*].artifact` に署名付き URL を生成。CLI はこの情報を `publish.log` と `CliDiagnosticEnvelope.summary.stats` に書き込む。

## 4. 配布ポリシー

- 名前空間、転送制御、ライセンス要件。
- プライベート・ミラー・オンプレミス展開のガイドライン。
- リージョンごとのデータ保持とコンプライアンス。

## 5. 検索・リコメンド

- DSL カテゴリ、capability、効果タグに基づく検索フィルタ。
- ダウンロード数、互換性レポート、性能ヒントの公開。

## 6. セキュリティ・監査

- 公開フックと `AuditCapability` を用いた完全証跡。
- 脆弱性通報フロー（registry -> CLI -> manifest）。
- TUF/Update Framework 互換性検討。

## 7. ロードマップ項目

- Phase 2 (6-12ヶ月) での MVP 実装タスク一覧化。
- 将来の AI 支援レビュー、自動互換性チェック（4-3, 4-5 参照）。

> メモ: 詳細な API リファレンス、エラーモデル、サンプルレスポンスは今後追加。Chapter 4 全体の完成度に応じて正式版へ格上げする。
