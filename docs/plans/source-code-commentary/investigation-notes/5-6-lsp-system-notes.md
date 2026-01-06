# 調査メモ: 第18章 LSP/システム補助

## 対象モジュール

- `compiler/runtime/src/lsp/mod.rs`
- `compiler/runtime/src/lsp/derive.rs`
- `compiler/runtime/src/lsp/embedded.rs`
- `compiler/runtime/src/parse/embedded.rs`
- `compiler/runtime/src/system/mod.rs`
- `compiler/runtime/src/system/env.rs`
- `compiler/runtime/src/system/process.rs`
- `compiler/runtime/src/system/signal.rs`
- `compiler/runtime/src/system/daemon.rs`
- `compiler/runtime/src/system/audit.rs`

## 入口と全体像

- `lsp` は Core.Lsp の最小実装として、LSP 型と JSON-RPC の簡易ヘルパを提供する。`derive` と `embedded` をサブモジュールとして公開する。
  - `compiler/runtime/src/lsp/mod.rs:1-13`
- `derive` は `Core.Parse` のメタデータと観測トークンから、補完・アウトライン・セマンティックトークン・ホバー情報を自動生成する。
  - `compiler/runtime/src/lsp/derive.rs:4-120`
- `embedded` は埋め込み DSL の span と LSP サーバーのルーティング表を持ち、入力位置から該当サーバーを引く。
  - `compiler/runtime/src/lsp/embedded.rs:5-50`
- `parse/embedded.rs` の `EmbeddedDslSpec` は `lsp: Option<LspServer>` を持ち、DSL ごとの LSP ハンドラを登録できる。
  - `compiler/runtime/src/parse/embedded.rs:9-79`
- `system` は OS 連携の標準 API を提供するが、現状は Capability 検証とエラー整形が中心で、実処理は未配線の箇所が多い。
  - `compiler/runtime/src/system/mod.rs:1-6`

## データ構造

### LSP

- `Position`/`Range` は 0-based の位置情報を表す。
  - `compiler/runtime/src/lsp/mod.rs:15-27`
- `LspCapabilities`/`LspServer` が機能フラグと最小サーバー表現を持つ。
  - `compiler/runtime/src/lsp/mod.rs:29-58`
- `LspDiagnostic` と `DiagnosticSeverity` が診断表現を提供する。
  - `compiler/runtime/src/lsp/mod.rs:61-77`
- `JsonRpcMessage`/`LspError`/`LspErrorKind` が JSON-RPC デコード結果とエラー種別を定義する。
  - `compiler/runtime/src/lsp/mod.rs:79-106`
- `DeriveModel` と `LspDeriveEnvelope` が LSP 自動導出の集約モデルを表す。
  - `compiler/runtime/src/lsp/derive.rs:9-67`
- `CompletionItem`/`OutlineNode`/`SemanticToken`/`HoverEntry` が出力要素を構成する。
  - `compiler/runtime/src/lsp/derive.rs:17-40`
- `EmbeddedLspRoute`/`EmbeddedLspRegistry` が span と DSL ID を使った LSP ルーティングを表す。
  - `compiler/runtime/src/lsp/embedded.rs:5-35`

### System

- `PlatformInfo`/`EnvContext`/`EnvError` が環境変数 API の文脈情報と失敗種別を保持する。
  - `compiler/runtime/src/system/env.rs:8-107`
- `Command`/`SpawnOptions`/`ProcessHandle`/`ProcessError` がプロセス操作の入力・戻り値・エラーを表す。
  - `compiler/runtime/src/system/process.rs:22-107`
- `SignalPayload`/`SignalDetail` がシグナル情報の拡張 payload を持つ。
  - `compiler/runtime/src/system/signal.rs:20-43`
- `DaemonConfig` がデーモン化の入力パラメータを保持する。
  - `compiler/runtime/src/system/daemon.rs:4-10`
- `ProcessAuditEvent`/`SignalAuditEvent` と各 Info が監査ログに書き込むための型を提供する。
  - `compiler/runtime/src/system/audit.rs:12-89`

## コアロジック

### LSP ヘルパ

- `position`/`range` は負値を `0` にクリップして LSP 位置を生成する。
  - `compiler/runtime/src/lsp/mod.rs:109-123`
- `to_lsp` は `GuardDiagnostic` の severity/code を LSP 診断へ移す。
  - `compiler/runtime/src/lsp/mod.rs:139-151`
- `encode_publish` は `textDocument/publishDiagnostics` の JSON をシリアライズする。
  - `compiler/runtime/src/lsp/mod.rs:154-165`
- `decode_message` は JSON から method と params を抽出し、失敗時は `LspErrorKind::DecodeFailed` を返す。
  - `compiler/runtime/src/lsp/mod.rs:167-193`

### Lsp.Derive

- `collect_with_source` は `ParseState` を生成してパーサを実行し、`ParseMetaRegistry` と `ObservedToken` から `DeriveModel` を組み立てる。
  - `compiler/runtime/src/lsp/derive.rs:80-120`
- `collect_completions` は `keyword`/`symbol` のみを抽出して重複排除し、安定ソートする。
  - `compiler/runtime/src/lsp/derive.rs:122-141`
- `collect_outline` は `ParserMetaKind::Rule` を木構造に変換し、循環参照をガードする。
  - `compiler/runtime/src/lsp/derive.rs:143-203`
- `collect_hovers` は doc コメントが付いた rule/token のみを収集する。
  - `compiler/runtime/src/lsp/derive.rs:205-221`
- `range_from_span` が `Span` を 0-based LSP 範囲に変換する。
  - `compiler/runtime/src/lsp/derive.rs:224-232`

### LSP ルーティング

- `register_route` が span と DSL ID を登録し、`resolve_route` は入力位置から一致するルートを返す。
  - `compiler/runtime/src/lsp/embedded.rs:17-35`

### System API と Capability

- `process::spawn`/`wait`/`kill` は Capability 検証後に `Unsupported` を返し、実装は未配線。
  - `compiler/runtime/src/system/process.rs:151-176`
- `ensure_process_capability` は `StageRequirement::AtLeast(Experimental)` で `core.process` を検証する。
  - `compiler/runtime/src/system/process.rs:178-185`
- `signal::send`/`wait`/`raise` も同様に Capability を検証して未配線エラーを返す。
  - `compiler/runtime/src/system/signal.rs:46-75`
- `get_env`/`set_env`/`remove_env` は標準ライブラリの環境変数 API に委譲し、エラーに文脈情報を付加する。
  - `compiler/runtime/src/system/env.rs:77-107`
- 監査メタデータ生成は `insert_process_audit_metadata` と `insert_signal_audit_metadata` に集約される。
  - `compiler/runtime/src/system/audit.rs:37-135`

## エラー処理

- `decode_message` は JSON の `method` 欠落を `DecodeFailed` として扱う。
  - `compiler/runtime/src/lsp/mod.rs:167-179`
- `ProcessError` は `IntoDiagnostic` を実装し、拡張情報と監査メタデータを付与する。
  - `compiler/runtime/src/system/process.rs:109-147`
- `system.capability.missing` の診断コードは missing capability メッセージに対してのみ付与される。
  - `compiler/runtime/src/system/process.rs:188-197`
- `SignalError` は Capability 失敗時に `SignalErrorKind::Unsupported` を返す。
  - `compiler/runtime/src/system/signal.rs:70-75`
- `EnvError` は UTF-8 でない環境変数を `InvalidEncoding` として扱う。
  - `compiler/runtime/src/system/env.rs:77-88`

## 仕様との対応メモ

- LSP の型とヘルパは `docs/spec/3-14-core-lsp.md` に対応するが、JSON-RPC ループは未実装で最小機能に留まる。
- System は `docs/spec/3-18-core-system.md` を参照し、Process/Signal/Daemon の API 形状を先行定義している。
- `Lsp.Derive` は `Core.Parse` のメタデータ利用が前提で、`docs/spec/2-2-core-combinator.md` と `docs/spec/2-7-core-parse-streaming.md` の doc/comment 方針に依存する。
