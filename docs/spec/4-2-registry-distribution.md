# 4.2 Registry & Distribution

> 目的：Reml パッケージレジストリおよび配布モデル（分散・中央ハイブリッド）の仕様を定義し、`reml publish` コマンドや依存解決の基盤を整備する。

## 0. 仕様メタデータ

| 項目 | 内容 |
| --- | --- |
| ステータス | 執筆中（Working Draft） |
| 参照文書 | [reml-ecosystem-analysis.md](reml-ecosystem-analysis.md) §2, §4.2, §5.2 |
| 関連章 | [3-6-core-diagnostics-audit.md](3-6-core-diagnostics-audit.md), [3-7-core-config-data.md](3-7-core-config-data.md), [3-8-core-runtime-capability.md](3-8-core-runtime-capability.md), [4-1-package-manager-cli.md](4-1-package-manager-cli.md) |

## 1. レジストリアーキテクチャ

### 1.1 ハイブリッド構成
- 中央レジストリ（仮称 `registry.reml-lang.org`）が**正規メタデータ**と署名を管理し、地理的に分散したミラーが静的アーティファクトを配布する。
- クライアントは `registry.json`（`~/.reml/registries`）に定義された優先順でエンドポイントを解決する。ネットワーク障害時はミラーへフェイルオーバーし、`CliDiagnosticEnvelope.summary.stats.registry_failover` を更新する。

### 1.2 API 境界
- メタデータ API は REST ベースで `/v1/packages`, `/v1/search`, `/v1/advisories` を提供。大規模クエリには GraphQL エンドポイント `/graphql` を提供し、IDE が DSL 情報を引き出せるようにする。
- 認証は Personal Access Token（PAT）と OIDC をサポートし、`registry login` が OIDC コードフローを実装する。PAT は `~/.reml/credentials/<registry>.json` に暗号化保存する。

### 1.3 耐障害性
- 重要なメタデータ（パッケージ一覧、バージョン、アドバイザリ）は TUF（The Update Framework）互換のメタファイルで署名され、ローカルキャッシュが改竄を検知できる仕組みを導入する。
- CDN レイヤは静的資産（`.remlpkg`、ドキュメント）に利用し、オリジンサーバーの負荷を軽減する。クライアントは `ETag` と `If-None-Match` を使用して差分同期を行う。

## 2. パッケージメタデータ

### 2.1 基本構造
- `GET /v1/packages/{name}/{version}` のレスポンスは以下の JSON スキーマに従う。

```
{
  "name": "reml/std-pipeline",
  "version": "1.2.0",
  "summary": "Pipeline DSL primitives",
  "description": "...",
  "license": "MIT",
  "targets": [...],
  "dependencies": [...],
  "dsl": {
    "capabilities": ["core", "pipeline"],
    "exports": ["pipeline::compose"]
  },
  "advisories": [...],
  "signatures": {
    "package": "...",
    "artifacts": {"desktop-x86_64": "..."}
  }
}
```

- `dependencies` は `name`, `constraint`, `kind (runtime|build|dev)`, `optional` を持つ。
- `dsl` ブロックは `DslCapabilityProfile`（[3-8 §7](3-8-core-runtime-capability.md#dsl-capability-utility)）と照合され、互換性判定の根拠となる。

### 2.2 ターゲットメタデータ

| フィールド | 型 | 説明 |
| --- | --- | --- |
| `profile_id` | `Str` | `TargetProfile.id`。`reml target list` と一致。 |
| `triple` | `Str` | LLVM triple（例：`x86_64-unknown-linux-gnu`）。`RunArtifactMetadata.llvm_triple` と突合する。 |
| `runtime_revision` | `Str` | ランタイム互換リビジョン。CLI 側のランタイムと一致する必要がある。 |
| `stdlib_version` | `SemVer` | バンドル済み標準ライブラリのバージョン。`Core.Env.resolve_run_config_target` が検証。 |
| `capabilities` | `List<Str>` | `capability_name(TargetCapability::…)` で列挙した機能集合。 |
| `artifact` | `Url` | 当該ターゲット用アーティファクト（`.remlpkg`, `staticlib` 等）への署名付き URL。 |
| `hash` | `Digest` | `RunArtifactMetadata.hash`。`sha256:` などアルゴリズムのプレフィクスを含む。 |
| `format` | `Str` | `"remlpkg"`, `"staticlib"`, `"cdylib"` 等。MIME の判定に用いる。 |
| `signature` | `Optional<Str>` | Ed25519/Minisign 等の署名値。

- レジストリは `profile_id` ごとにインデックス化し、CLI からの `GET` 要求に対して最適なターゲットを返す。未提供の場合は `targets = []` とし、`requires_source_build = true` を明示する。

### 2.3 セキュリティ情報
- `advisories` は `id`, `severity`, `affected`, `patched`, `references` を含む。CLI は `reml add` 実行中に警告を表示する。
- `signatures.package` はマニフェスト（`reml.toml` ベースのメタデータ）の署名で、`publish` クライアントが署名者情報を `AuditEnvelope` に記録する。

## 3. 公開ワークフロー

### 3.1 アップロードシーケンス
1. `reml publish` が `build` 成果物を検証し、`RunArtifactMetadata` を収集する。
2. メタデータを `POST /v1/publish` へ送信し、レジストリが `runtime_revision`, `capabilities`, `hash` を検証する。
3. レジストリはアーティファクトの PUT URL を返し、クライアントが直ちにアップロードする。アップロード完了後、`publish_receipt` が返される。

### 3.2 検証規則
- ランタイム互換性：レジストリは各ターゲットの `runtime_revision` をチャネル別互換表と照合し、不一致は `409 target.abi.mismatch` を返す。
- Capability 検証：レジストリが把握していない capability 名が含まれる場合は `400 target.capability.unknown` を返す。将来の拡張に備え、`custom_capabilities` フィールドで事前登録済みカスタム capability のみ許容する。
- 再現性：`hash` が既存アーティファクトと一致する場合は `409 artifact.duplicate`。`--allow-reuse` 指定時のみ上書きを許容し、過去の履歴を保持する。
- 署名：署名が添付されている場合、公開鍵チェーンを `Key Transparency Log` に照合し、結果を `AuditEnvelope.metadata["signature"]` に書き込む。

### 3.3 ロックファイル連携
- レジストリは `ResolvedDependencyGraph` のサマリを `publish_receipt.lock_digest` として返却し、クライアントは `reml.lock` の整合性チェックに利用する。

## 4. 配布ポリシー

### 4.1 名前空間
- 公式名前空間 `reml/*` はコアチームが管理し、コミュニティパッケージは `community/*`, `org.<name>/*` などのプリフィクスを使用する。
- 予約語（`core`, `std`, `experimental`）は `../guides/community-handbook.md` で定義されたルールに従い配布制限をかける。

### 4.2 アクセス制御
- プライベートレジストリは `visibility = private` を設定し、`registry login` がアクセストークンを取得できない場合は 401 を返す。CLI は `--require-auth` 指定時に未ログインなら即時失敗する。
- ミラーリングは `mirror_of` フィールドで関連付け、差分同期は `snapshot`/`targets` メタファイルで管理する。

### 4.3 ライセンスと法令遵守
- レジストリは `license` フィールドを必須化し、互換性のないライセンス組み合わせが検出された場合は `publish` をブロックする。組み合わせ判定は `license-policy.json` に基づき、更新時はレジストリから CLI へ通知される。
- データ保持とコンプライアンス要件（GDPR 等）はリージョンごとに `retention_policy` に記録し、ユーザーは API で参照できる。

## 5. 検索・リコメンド

### 5.1 検索 API
- `GET /v1/search?q=<query>&capability=<cap>&effect=<tag>` により名前・説明・DSL capability を横断検索する。
- 結果は `score`, `downloads_last_30d`, `compatibility_level` を含む。`compatibility_level` は `Stable`, `Preview`, `Experimental` の 3 段階で `publish` 時に指定する。

### 5.2 レコメンド指標
- CLI は `reml add --suggest` で互換可能な DSL を提示し、レジストリは `similar_packages` を返す。類似度計算は capability、effect タグ、ダウンロード傾向を組み合わせて算出する。
- コミュニティ評価（スター、レビュー）は `moderated` フィールドで検閲済みかを明示する。

## 6. セキュリティ・監査

### 6.1 監査イベント
- レジストリは `publish`, `yank`, `advisory` を監査イベントとして保管。`AuditCapability`（[3-6](3-6-core-diagnostics-audit.md)）と互換の JSON 形式で CLI から取得できる。
- 重大アドバイザリが公開された場合、レジストリは `registry advisory push` の Webhook を発行し、CLI は `reml audit fetch`（計画中）で取り込む。

### 6.2 脆弱性通報
- 通報フロー：レポーター → セキュリティチーム → 初期対応（72 時間以内） → 修正告知 → 公開。レジストリは `security@reml-lang.org` の PGP キーを提供し、機密保持を行う。
- 既知脆弱性は `GET /v1/advisories/{id}` で公開され、`affected` セクションに `package`, `version_range`, `patched` を含む。

### 6.3 TUF 互換性検討
- Phase 2 では TUF の `root`, `snapshot`, `targets`, `timestamp` メタファイルを採用し、Phase 3 でローテーション自動化を目指す。

## 7. ロードマップ
- **Phase 1 (0-6ヶ月)**：メタデータ API、`publish` パイプライン、署名検証、基本検索機能を実装。
- **Phase 2 (6-12ヶ月)**：ミラー展開、TUF メタファイル、アドバイザリ配信を導入。CLI と IDE の検索統合を行う。
- **Phase 3 (12-18ヶ月)**：AI 支援レビュー、自動互換性チェック、ダウンロードトレンドのダッシュボード化を行う。

> メモ: API リファレンス、JSON スキーマ、サンプルレスポンスは別添付録として整理予定。`../guides/cli-workflow.md` の更新と同期し、CLI 実装との相互参照を強化する。
