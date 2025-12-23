# Reml 標準ライブラリ拡張の調査

## 1. はじめに

本文書は、Reml を実用的で「一人前（full-fledged）」のプログラミング言語にするための、標準ライブラリ拡張に関する調査と提案をまとめたものです。現在の Reml 標準ライブラリ（`Core.*`）は、言語実装ドメイン（パーサー、診断、テキスト、ソースコード処理）に重点を置いていますが、汎用言語としてウェブサーバー、システムツール、データ処理アプリケーションなどのタスクに対応するためには、より広範なユーティリティが必要です。

## 2. 標準ライブラリの調査

人気のある「バッテリー同梱（batteries-included）」型言語（Python, Go）やシステム言語（Rust）との標準ライブラリ機能の比較。

| 機能ドメイン | Reml (現状) | Python | Go | Rust (std + 公式/事実上の標準) |
| :--- | :--- | :--- | :--- | :--- |
| **コレクション** | `Core.Collections` (Map, Set, List) | `list`, `dict`, `set`, `collections` | `slice`, `map` | `std::collections` |
| **テキスト/Unicode** | `Core.Text` (Grapheme, Unicode) | `str` (Unicode), `re` | `strings`, `unicode/utf8` | `String`, `regex` (crate) |
| **ファイル/IO** | `Core.Io`, `Core.Path` | `os`, `pathlib`, `io` | `os`, `io`, `path/filepath` | `std::fs`, `std::io`, `std::path` |
| **ネットワーク** | **欠落** | `socket`, `http`, `urllib`, `asyncio` | `net`, `net/http` | `std::net`, `reqwest/hyper` (crates) |
| **暗号化** | **欠落** | `hashlib`, `secrets`, `ssl` | `crypto/sha256`, `crypto/rand` | `ring`, `rust-crypto` (crates) |
| **圧縮** | **欠落** | `gzip`, `zipfile`, `tarfile` | `compress/gzip`, `archive/zip` | `flate2`, `tar` (crates) |
| **プロセス/OS** | Plugin (`4-2`) / `Core.Env` | `subprocess`, `os`, `sys` | `os/exec`, `os` | `std::process`, `std::env` |
| **エンコーディング** | `Core.Config` (JSON/TOML), Base64 in Text | `json`, `csv`, `base64` | `encoding/json`, `encoding/csv` | `serde` (crate), `base64` (crate) |
| **日付/時間** | `Core.Numeric`/`Core.Time` | `datetime`, `time`, `zoneinfo` | `time` | `std::time`, `chrono` (crate) |
| **数学** | `Core.Numeric` (基本のみ) | `math`, `random`, `decimal` | `math`, `math/rand` | `std` (basic), `rand` (crate) |
| **CLI/ターミナル** | **欠落** | `argparse`, `readline`, `shlex` | `flag`, `bufio` | `clap`, `rustyline` (crates) |
| **ファイルシステム補助** | **欠落** | `glob`, `pathlib`, `tempfile`, `watchdog` | `path/filepath`, `io/fs` | `walkdir`, `notify`, `tempfile` (crates) |
| **観測/ロギング** | `Core.Diagnostics` (重厚) | `logging`, `tracing` | `log`, `expvar` | `log`, `tracing` (crates) |
| **バイナリ/シリアライズ** | **欠落** | `pickle`, `struct`, `msgpack` | `encoding/gob`, `encoding/binary` | `serde`, `bincode`, `prost` (crates) |
| **テスト/検証** | **欠落** | `unittest`, `pytest` (外部) | `testing` | `std::test`, `proptest` (crates) |

## 3. ギャップ分析

### 3.1 ネットワーク (`Core.Net`)
**ステータス**: 完全に欠落。
**要件**: Webサービス、APIと対話するツール、またはネットワークアプリケーションを構築するために、Remlには以下が必要です：
- **HTTP クライアント/サーバー**: リクエストとコンテンツ提供のための高レベルAPI。
- **低レベルソケットアクセス**: 低レベルプロトコルのためのTCP/UDP。
- **非同期統合**: `Core.Async` と統合されている必要があります。

### 3.2 暗号化 (`Core.Crypto`)
**ステータス**: 完全に欠落。
**要件**: セキュリティ、認証、データの整合性に不可欠です。
- **ハッシュ**: SHA256, SHA512 など。
- **乱数**: 暗号論的擬似乱数生成器 (CSPRNG)。
- **暗号化**: 基本的な対称鍵（AES）および潜在的に非対称鍵のサポート。

### 3.3 圧縮・アーカイブ (`Core.Archive`)
**ステータス**: 欠落。
**要件**: パッケージングツール、データ処理、ネットワーク転送の最適化に必要です。
- **アルゴリズム**: Gzip, Deflate, Zstd (現代の標準)。
- **フォーマット**: Zip, Tar。

### 3.4 OSインタラクション (`Core.System` / `Core.Process`)
**ステータス**: `Core.Env` は存在。`Core.Process` はオプションのプラグイン（`4-2`）として定義されている。
**要件**: 汎用言語として、サブプロセスの生成は単なるプラグインではなくコア機能であるべきです。シグナル、パイプ、プロセスライフサイクルの制御は極めて重要です。

### 3.5 エンコーディング (`Core.Encoding`)
**ステータス**: 散在している。Base64 は `Core.Text` に、JSON/TOML は `Core.Config` にある。
**要件**: データフォーマット専用の名前空間と、`Core.Data` との責務分離。
- **CSV**: データサイエンスや交換に不可欠。
- **URL**: クエリパラメータのエンコード/デコード（多くの場合 Net の一部）。
- **Hex**: バイナリから16進数文字列への変換。
- **統一インターフェース**: ストリーミングエンコード/デコードに `Reader`/`Writer` トレイトを使用。
- **責務境界**: `Core.Encoding` は Codec とストリーミング変換、`Core.Data` はスキーマ・検証・操作に集中。

### 3.6 データベース (`Core.Sql`)
**ステータス**: 欠落。
**要件**: 完全なドライバは通常外部にありますが、バックエンドを交換可能にするための標準インターフェース（Goの `database/sql` や Javaの JDBC のようなもの）が必要です。

### 3.7 CLI/ターミナル (`Core.Cli`, `Core.Terminal`)
**ステータス**: 欠落。
**要件**: 日常的なツールやサーバー運用のために必須。CLI と対話入力は「言語の実用性」を左右するため、`Core.Diagnostics` と連携した一貫した UX が必要。
- **引数/サブコマンド**: `--help` 自動生成、サブコマンド、バリデーション。
- **プロンプト/補完**: 履歴、行編集、TTY 判定。
- **出力整形**: 色、幅、テーブル、プログレス表示。

### 3.8 ファイルシステム補助 (`Core.Fs`)
**ステータス**: 欠落。
**要件**: 多数のファイルを扱うツールやビルドを支える基盤。
- **探索/グロブ**: `walk`, `glob`, `ignore` 連携。
- **テンポラリ**: `TempDir`, `TempFile`。
- **監視**: 変更検知 (`watch`) を `Core.Async` へストリーム接続。

### 3.9 観測/ロギング (`Core.Log`, `Core.Metrics`, `Core.Trace`)
**ステータス**: `Core.Diagnostics` はあるが、アプリ向けの簡易 API が不足。
**要件**: 実運用での可観測性を確保し、監査ポリシーと衝突しない軽量 API を提供する。
- **Logging**: 低コストのレベル別ログ。
- **Metrics**: カウンタ、ゲージ、ヒストグラム。
- **Tracing**: リクエスト単位のスパンと相関 ID。

### 3.10 バイナリ/シリアライズ (`Core.Serialization`)
**ステータス**: 欠落。
**要件**: RPC、キャッシュ、IPC に不可欠。`Core.Encoding` との責務分離を前提にする。
- **フォーマット**: CBOR、MessagePack、Bincode、Protobuf（後者はオプションでもよい）。
- **方針**: `Core.Data` のスキーマと連携しつつ、ストリーミング API を提供する。

### 3.11 同期/並行プリミティブ (`Core.Sync`)
**ステータス**: `Core.Async` は計画されているが、同期プリミティブの仕様が欠落。
**要件**: マルチスレッドやタスク間の安全な共有と調整。
- **Primitives**: `Mutex`, `RwLock`, `Condvar`。
- **Channels**: `mpsc`, `broadcast`。
- **Atomics**: `AtomicInt`, `AtomicBool` 等のロックフリー操作。

### 3.12 デーモン/サービス化 (`Core.System.Daemon`)
**ステータス**: 欠落。
**要件**: 長時間稼働するサービスの運用支援。
- **機能**: PID ファイル、デーモン化、シグナルハンドリング、シャットダウンフック。

### 3.13 テスト/検証 (`Core.Test`)
**ステータス**: 欠落。
**要件**: 言語レベルの最小テストハーネスと診断連携。
- **機能**: `test`, `expect`, `assert_eq`、フィクスチャのスコープ管理。

## 4. 標準ライブラリ拡張の提案

Reml 標準ライブラリトラックに以下のモジュールを追加することを提案します。

### 4.1 `Core.Net`
**効果**: `effect {net}` (`io`, `io.async` を包含)
**サブモジュール**:
- `Core.Net.Http`: `Client`, `Server`, `Request`, `Response`。
- `Core.Net.Tcp`: `TcpListener`, `TcpStream`。
- `Core.Net.Udp`: `UdpSocket`。
- `Core.Net.Url`: `Url` パーサー/ビルダー。

### 4.2 `Core.Crypto`
**効果**: `effect {crypto}` (計算), `effect {random}`
**サブモジュール**:
- `Core.Crypto.Hash`: `Sha256`, `Sha512`, `Blake3`。
- `Core.Crypto.Random`: `secure_random_bytes`, `random_u64`。
- `Core.Crypto.Cipher` (オプション/将来): AES-GCM, ChaCha20。

### 4.3 `Core.Archive`
**効果**: `effect {io}` (ストリーミング)
**サブモジュール**:
- `Core.Archive.Gzip`: `GzipEncoder`, `GzipDecoder`。
- `Core.Archive.Zip`: `ZipReader`, `ZipWriter`。
- `Core.Archive.Tar`: `TarReader`, `TarWriter`。

### 4.4 `Core.Encoding`
**効果**: `@pure` (主に) または `effect {io}` (ストリーミング用)
**サブモジュール**:
- `Core.Encoding.Csv`: `Reader`, `Writer`。
- `Core.Encoding.Base64`: Text から移動/エイリアス。
- `Core.Encoding.Hex`: Hex ダンプ/ロード。
- `Core.Encoding.Json`: `Core.Config` へのエイリアス/参照、または特化したデータバインディング。
**責務境界**:
- `Core.Encoding`: Codec とストリーミング変換 (`JsonEncoder`, `Base64Decoder`) に集中。
- `Core.Data`: スキーマ定義、バリデーション、データ操作に集中。

### 4.5 `Core.System` (`Core.Env` と `Core.Process` の統合)
**効果**: `effect {process}, effect {system}`
**変更点**:
- `4-2-process-plugin` を標準ライブラリの `Core.System.Process` 配下に昇格させる。
- `Core.Env` を `Core.System.Env` に統合する（または `Core.Env` エイリアスとして維持する）。
**サブモジュール**:
- `Core.System.Daemon`: PID ファイル、デーモン化、シグナル/シャットダウンフック。

### 4.6 `Core.Math` (`Core.Numeric` の拡張)
**効果**: `@pure`
**機能**:
- 三角関数 (`sin`, `cos`, `tan`)。
- 対数 (`log`, `ln`)。
- 定数 (`PI`, `E`)。
- PRNG (非暗号ランダム) を `Core.Math.Random` に。

### 4.7 `Core.Cli`
**効果**: `@pure`（解析）/ `effect {io.console}`（実行支援）
**機能**:
- `ArgSpec`, `CommandSpec`, `ArgValue` による宣言的な引数定義。
- `parse_args`, `render_help`, `validate_args` の分離。
- `Core.Diagnostics` との統合エラー表示。

### 4.8 `Core.Terminal`
**効果**: `effect {io.console}`
**機能**:
- 文字装飾、幅計測、色/スタイル。
- `read_line`, `read_password`, `prompt_select`。
- TTY 判定や非 TTY 時のフォールバック。

### 4.9 `Core.Fs`
**効果**: `effect {io, system}` / `effect {io.async}`（監視）
**機能**:
- `walk`, `glob`, `copy_tree`。
- `TempDir`, `TempFile` と安全な自動削除ポリシー。
- `watch(path) -> Stream<FsEvent>` を `Core.Async` と接続。

### 4.10 `Core.Observability`
**効果**: `effect {diagnostic, audit}`（既定）/ `effect {io}`（外部出力）
**サブモジュール**:
- `Core.Log`: `Debug/Info/Warn/Error` と直感的 API を提供する軽量ファサード。
- `Core.Metrics`: `counter`, `gauge`, `histogram`。
- `Core.Trace`: `Span`, `TraceId` を `Core.Diagnostics` へ橋渡し。
**統合方針**:
- `Core.Diagnostics`/`AuditSink` と接続可能にし、既定は標準出力/標準エラーへ構造化テキスト出力。

### 4.11 `Core.Serialization`
**効果**: `@pure` / `effect {io}`（ストリーミング）
**サブモジュール**:
- `Core.Serialization.Cbor`, `Core.Serialization.MessagePack`。
- `Core.Serialization.Bincode`（高速バイナリ）。
- `Core.Serialization.Protobuf`（オプション、Stage 管理前提）。

### 4.12 `Core.Sync`
**効果**: `@pure`（構造）/ `effect {async}`（待機・通知）
**機能**:
- `Mutex`, `RwLock`, `Condvar`、`AtomicInt` 等。
- `mpsc`, `broadcast` のチャネル群。
- `Core.Async` と同期/非同期の両方で使えるインターフェースを整理。

### 4.13 `Core.Test`
**効果**: `@pure` / `effect {diagnostic}`（レポート）
**機能**:
- `test`, `expect`, `assert_eq`、フィクスチャのスコープ管理。
- `Core.Test.Property` を段階導入。

## 5. 実装戦略

1.  **Rust クレートの統合**: Reml の公式ランタイムは Rust ベースです。これらのモジュールは、パフォーマンスと信頼性を確保するために、可能な限り成熟した Rust クレートに直接マッピングすべきです。
    - `Core.Net` -> `reqwest`, `tokio`
    - `Core.Crypto` -> `ring`
    - `Core.Archive` -> `flate2`, `tar`, `zip`
    - `Core.Encoding` -> `csv`, `serde_json`, `base64`
    - `Core.Sync` -> `std::sync`, `tokio::sync`
    - `Core.Log` -> `log`, `tracing`

2.  **監査と安全性**:
    - すべての IO/Net/Process 操作は `Core.Diagnostics` および `AuditSink` と統合する必要があります。
    - `effect` システムタグは厳密に強制されなければなりません。`effect {net}` は一般的な `effect {io}` と区別し、きめ細かな Capability 制御（例：「ファイル IO は許可するがネットワークは拒否」）を可能にする必要があります。

3.  **クロスプラットフォーム**:
    - API は `Core.Path` を受け入れ、OS の差異（Windows パス vs POSIX）を処理する必要があります。
    - `Core.System` は、シグナルやプロセス属性のための抽象インターフェースを提供する必要があります。

## 着手順の目安

標準ライブラリ拡張の着手順を以下に整理する。実用性・安全性・段階的導入（`docs/spec/0-1-project-purpose.md`）を最優先の判断基準とする。

1. **P0: Core.Net / Core.Crypto**  
   - **理由**: 実用用途の根幹（API 連携・配信・認証/整合性）であり、欠落がボトルネック。  
   - **前提**: `Core.Async` と `Core.Io`/`Core.Path` 連携、`effect {net}`/`effect {crypto, random}` の監査統合。

2. **P1: Core.System（Process/Env/Daemon）/ Core.Sync**  
   - **理由**: サブプロセス・シグナル・同期原語は汎用言語として必須。`Core.Process` をプラグインから昇格。  
   - **前提**: Capability Stage と診断監査の整合（`docs/spec/3-6-core-diagnostics-audit.md` / `3-8-core-runtime-capability.md`）。

3. **P2: Core.Encoding / Core.Serialization / Core.Archive / Core.Math**  
   - **理由**: データ交換・パッケージング・数値処理の実務要件を満たすため。  
   - **前提**: `Core.Data` との責務境界（`docs/spec/3-7-core-config-data.md`）を明確化し、ストリーミング API を `Reader`/`Writer` と統合。

4. **P3: Core.Fs / Core.Terminal / Core.Observability**  
   - **理由**: ツール運用・可観測性・開発体験の向上。`Core.Diagnostics` との住み分けを保ちつつ簡易 API を提供。  
   - **前提**: `Core.Async` ストリーム接続、`effect {io.console}` と監査ログの統一。

5. **P4: Core.Sql / Core.Test 拡張**  
   - **理由**: 実装コストが大きく、外部ドライバ依存が強い。インターフェース標準化から着手。`Core.Test` は Phase 4 で最小実装済みのため拡張は後続。  
   - **前提**: Stage 管理とプラグイン/FFI ポリシー（`docs/spec/3-8-core-runtime-capability.md`）の再確認。

## 6. 次のステップ

1.  基盤的な可用性のブロッカーであるため、`Core.Net` と `Core.Crypto` を優先する。
2.  `Core.System`（`Process`/`Daemon`/`Env`）と `Core.Sync` の仕様を先行ドラフトする。
3.  `Core.Net` の詳細仕様を `Core.Io` スタイルに合わせて起草し、`Core.Async` との接合点を明確化する。
4.  `Core.Encoding` と `Core.Data` の責務境界を [3-7 Core Config & Data](../spec/3-7-core-config-data.md) と整合させる。
5.  `Core.Test` の最小テストハーネスを定義し、`Core.Diagnostics` へのレポート形式を決める。

## 7. 準標準ライブラリ候補の整理

標準ライブラリ本体の拡張と並行して、エコシステム側で事実上の標準になりやすい領域を整理し、準標準化の候補として監視する。

### 7.1 エコシステムで頻出の補助ライブラリ整理
他言語で「ほぼ標準」扱いになっている拡張ライブラリを整理し、標準化/準標準化候補を抽出する。
- **HTTP/ネットワーク**: Python `requests`、Go `net/http`、Rust `reqwest` に相当。
- **シリアライズ**: Rust `serde`、Go `encoding/json` に相当。
- **CLI**: Rust `clap`、Go `cobra` に相当。
- **ロギング/トレーシング**: Rust `tracing`、Go `zap` に相当。
- **設定/環境**: Rust `config`、Go `viper` に相当（`Core.Config` と整合）。

> **調査メモ**: `Core.Diagnostics` との責務境界は [3-6 Core Diagnostics & Audit](../spec/3-6-core-diagnostics-audit.md) を参照。`Core.Encoding` と `Core.Data` の切り分けは [3-7 Core Config & Data](../spec/3-7-core-config-data.md) に整合させる。`Core.Async` との連携は [3-9 Core Async](../spec/3-9-core-async-ffi-unsafe.md) を参照する。
