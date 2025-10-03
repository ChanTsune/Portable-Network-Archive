# experimental stdio BSD-tar互換 TODO

目的: `pna experimental stdio` が bsdtar(1) のオプション互換マトリクス（`ロードマップ.md`）に掲げた全項目を満たすこと。各タスクは実装内容と検証方法を明記し、完了後は対応するロードマップ行を "✅" に更新する。

---
## 0. 共通準備
- [x] 0-1 `bsdtar` 利用可否確認 (`bsdtar --version` を macOS/Linux で実行)。存在しない場合は libarchive をインストール。
- [x] 0-2 開発環境基準整備: `cargo fmt --check`, `cargo clippy --all-targets`, `cargo test --all` を実行し、ベースラインを保存。
- [x] 0-3 `tests/bats/libarchive/` を作成し、libarchive upstream (`https://github.com/libarchive/libarchive`) から `tar/test` スイートを同期する `scripts/sync-libarchive-tests.sh` を実装。submodule を導入するか README に取得手順を明記.
- [x] 0-4 ゴールデン比較用テンポラリディレクトリ（例: `tests/tmp/compat`）とクリーンアップルールを定義。

---
## 1. モード系オプション（ロードマップ §1.1）
- [x] 1-1 `-r/--append` 互換
  - 実装: `cli/src/command/stdio.rs` に `short = 'r'` を追加し、圧縮指定時はエラーにするガードを実装。
  - テスト: Bats で `-rf archive` と gzip 圧縮アーカイブへのエラーを比較。
- [x] 1-2 `-u/--update`
  - 実装: `StdioCommand` に `short = 'u'`; `run_stdio` から `command::update::UpdateCommand` を呼ぶラッパを追加。
  - テスト: Bats で `-uf` シナリオ（新旧ファイル）を作り、bsdtar と比較。
- [x] 1-3 `-d/--delete`
  - 実装: `StdioCommand` に `delete` フラグを追加し、`command::delete::DeleteCommand` を呼び出す。
  - テスト: Bats でエントリ削除、`bsdtar -df` との差分無しを確認。
- [x] 1-4 `-A/--append-to`
  - 実装: `concat` コマンドを stdio から利用できるブリッジを追加し、`-A` alias を登録。
  - テスト: 2 つのアーカイブを `-Af` で連結した結果を bsdtar と比較。
- [x] 1-5 `@archive` 入力
  - 実装: `StdioCommand.files` 解析時に `@` から始まる要素を処理し、`collect_split_archives` に委譲。
  - テスト: `-c -f - file @archive.tar` の挙動をゴールデン比較。
- [x] 1-6 `--get`
  - 実装: `--get` alias を `extract` に追加。
  - テスト: `pna experimental stdio --get` と `bsdtar --get` のヘルプ・挙動比較。

---
## 2. 入出力・探索オプション（ロードマップ §1.2）
### 2.1 I/O / 環境関連
- [x] 2-1 `-a/--auto-compress`
  - 実装: 出力ファイル名の拡張子から `CompressionAlgorithmArgs` を推定するユーティリティを実装。
  - テスト: `.tgz`, `.tar.gz`, `.tar.bz2`, `.tar.xz`, `.zip` などを比較。
- [x] 2-2 `-B/--read-full-blocks`
  - 実装: 読み込みループでブロックサイズ不足時に再読込するモードを追加（テープ I/O 用）。非対応の場合は「実装予定」の Issue を残さない。
  - テスト: 小さな `dd` で作成したテープファイルを利用し、bsdtar との挙動比較。
- [x] 2-3 `-b/--block-size`
  - 実装: `StdioCommand` に `block_size` (u16) を追加し、リーダ/ライタに 512-byte レコード数を渡す。
  - テスト: 20/32/64 ブロックサイズで tar を作成し、`bsdtar` の `--list --verbose` でヘッダ一致を確認。
- [ ] 2-4 `--chroot`
  - 現状: フラグを指定すると `--chroot is not supported yet` で異常終了する（`run_stdio` が `bail!`）。
  - TODO: ルート切替処理とエラーハンドリングを実装し、互換テストを追加。
- [x] 2-5 `--clear-nochange-fflags`
  - 実装: フラグを受理して警告ログを出し、互換性のために無視する（仕様に従い no-op として扱う）。
  - ドキュメント: `--help` とリリースノートで no-op である旨を明記。
- [x] 2-6 `--fflags`
  - 実装: 互換目的でフラグを受理し、警告を出して無視する。
  - ドキュメント: no-op 方針を反映した解説を追加する。
- [x] 2-7 `--format`
  - 実装: Clap で受理しつつ警告ログを出し、常に PNA フォーマットで処理（フォールバック）する。
  - ドキュメント: 指定しても PNA 以外に切り替わらないことを明示。
- [x] 2-8 `--options key=val`
  - 実装: フラグを受理し、指定値を警告付きで無視する（現状は解析のみ）。
  - TODO: 実際の圧縮/時刻オプションにマッピングする場合はテーブルを整備。
- [x] 2-9 `--use-compress-program`
  - 実装: オプションを受理して警告を出し、実際の外部コマンドは起動しない（互換上のダミー実装）。
  - ドキュメント: 無視される旨を `--help` に追記し、今後の改善項目として Issue をリンク。
  - テスト: 警告ログを検証する軽量テストを用意。

### 2.2 ファイル選択 / 再帰
- [ ] 2-11 `--exclude` / `--include`
  - 実装: `GlobPatterns` を BSD glob と fnmatch のルールに合わせる。ワイルドカード、否定、大小文字指定を対応。
  - テスト: libarchive テスト群でのフィルタケースや独自 Bats で比較。
- [ ] 2-12 `--exclude-vcs`
  - 現状: stdio / create / extract では `--unstable` なしで利用済みだが、`list` サブコマンドのみ ArgGroup で `--unstable` を要求。
  - TODO: list 側の `--unstable` 依存を解消し、VCS リストの最新化を継続。
  - テスト: 既存 Bats (`test_option_exclude_vcs.bats`) をゴールデン比較に移行し、list ケースも追加。
- [x] 2-13 `--null`
  - 実装: `read_paths` の NUL 区切り入力を bsdtar と同じ挙動に。
  - テスト: `printf 'a\0b\0'` などを `-T - --null` で比較。
- [x] 2-14 `-T/--files-from`
  - 実装: `short = 'T'` を追加し、`--files-from=-` をサポート。
  - テスト: NUL 有無を含む複数ケースをゴールデン比較。
- [x] 2-15 `-X/--exclude-from`
  - 実装: `short = 'X'`, `--unstable`制約解除。
  - テスト: exclude ファイルの正/誤判定を比較。
- [x] 2-16 `-k/--keep-old-files`
  - 実装: `OutputOption` に `keep_existing` を追加し、存在ファイルを保存。
  - テスト: 同名ファイルがある場合の挙動比較。
- [x] 2-17 `--keep-newer-files`
  - 実装: `OutputOption` にタイムスタンプ比較ロジック追加。
  - テスト: 古い/新しいファイルを用意し、上書き有無を比較。
- [ ] 2-18 `-L/--dereference` & `-h`
  - 現状: `--follow-links` 長オプションのみ提供。短縮 alias `-L`/`-h` は Clap 定義未追加。
  - TODO: Clap へ `short = 'L'` と `short = 'h'` を登録し、ヘルプ/テストを更新。
- [x] 2-19 `-l/--check-links`
  - 実装: `HardlinkTracker` でハードリンク参照数を追跡し、`ensure_hardlinks_complete` を `create`/`append`/`update` 経路に組み込み不足分を検出。
  - テスト: `ensure_hardlinks_complete` の単体テストで欠落ケースを再現しエラーとなることを確認。
- [x] 2-20 `--one-file-system`
  - 実装: `collect_items` で `WalkDir::same_file_system(true)` を有効化し、CLI 各モードからフラグを受け取るよう統合。
  - テスト: unit テストでフラグを指定した収集が成功することを確認 (`collect_items_one_file_system_flag`).
- [x] 2-21 `--nodump`
  - 実装: `collect_items` で `has_nodump_flag` を用いて nodump フラグ付きのパスを除外し、全パスから CLI に引き渡すよう対応。
  - テスト: `collect_items_skips_nodump_entries` で nodump フラグを付与したファイルが除外されることを確認。
- [x] 2-22 `--ignore-zeros`
  - 実装: アーカイブリーダーにゼロブロック跳過ロジックを導入し、`run_process_archive_with_options` から制御できるようにした。
  - テスト: 人為的にゼロパディングを挿入した PNA を用意し、`--ignore-zeros` 指定時のみ後続エントリを処理できることを検証。
- [x] 2-23 `-C/--cd`, `-H`, `--help`, `-f`, `--gid`, `--gname`
  - 実装状態を再確認し、`-H` の `--unstable` 依存を解除。クイック CLI テストで `-H` の受理を検証。
  - テスト: create サブコマンドの `-H` 指定が `--unstable` 無しで受理されることを確認。
- [x] 2-24 `--lrzip`, `--lz4`, `--lzma`, `--lzop`, `--zstd`
  - 実装: 既存 `--zstd` のままにしつつ、その他互換フラグは受理して警告の上 zstd にフォールバックする形で CLI を拡張。
  - テスト: `create_accepts_legacy_compression_flags` で互換フラグが成功することを確認。

---
## 3. 時刻・所有権・メタデータ（ロードマップ §1.4, §1.5）
- [x] 3-1 `--newer* / --older*` 系
  - 実装: `build_time_filters` でフィルタを構築し、`collect_items` と `update` 経路へ適用。
  - テスト: `collect_items_respects_mtime_filters` と `DateTime::from_system_time` の単体テストを追加。
- [x] 3-2 `-s` / `--transform`
  - 実装: `-s`/`--transform` のヘルプを安定化し、experimental stdio/create/append/update/extract 経路が共通の `PathTransformers` を利用。
  - テスト: `create/extract` 既存シナリオを安定化させ、`stdio` 用 substitution/transform 回帰テストを追加。
- [x] 3-3 `-p/--preserve-permissions` & `--no-same-permissions`
  - 実装: `-p` を各 CLI で安定化し、`--no-same-permissions` を追加して `KeepOptions` の `keep_permission` 計算に反映。
  - テスト: 既存 keep-permission 系テストの `--unstable` 依存を解消し、stdio 向けに `no_same_permissions` 回帰テストを追加。
- [x] 3-4 `-m/--modification-time`
  - 実装: 抽出経路に `--modification-time` を追加し、`OutputOption` がファイル/ディレクトリの mtime を現在時刻に設定するよう更新。
  - テスト: `cli/tests/cli/stdio/modification_time.rs` で `-p` と組み合わせた挙動を検証。
- [ ] 3-5 `-o` (extract as self)
  - 実装: `no_same_owner` と別に `extract_as_self` フラグを追加し、`chown` をスキップ。
  - テスト: root / 非 root 双方で比較。
- [x] 3-6 `--same-owner` / `--no-same-owner`
  - 実装: `OutputOption.same_owner` に反映し、`no_same_owner` 指定で抽出時に所有者を変更しない。
  - TODO: bsdtar との比較テストを追加してプラットフォーム差分を整理。
- [x] 3-7 `--numeric-owner`
  - 実装: `OwnerOptions::new` で数値 ID 優先に切り替え済み。
  - TODO: Numeric owner のゴールデン比較テストを作成。
- [x] 3-8 `--uid/--gid/--uname/--gname`
  - 実装: `OwnerOptions` へ橋渡し済みで create/extract 双方に適用。
  - TODO: 優先順位のテストを作成。
- [ ] 3-9 `--acls/--no-acls`
  - 実装: ACL 保存/除外トグルを `KeepOptions` に追加。
  - テスト: POSIX ACL を設定したファイルの round-trip を比較。
- [ ] 3-10 `--xattrs/--no-xattrs`
  - 実装: xattr の保存/除外制御を追加。
  - テスト: xattr を持つファイルの round-trip 比較。
- [ ] 3-11 `--hfsCompression`, `--mac-metadata`, `--nopreserveHFSCompression`
  - 実装: macOS プラットフォーム限定のオプションを調査・実装。
  - テスト: macOS で Finder 属性・リソースフォークの保存/復元を比較。
- [ ] 3-12 `--fflags/--no-fflags`
  - 実装＆テスト: 2-6 と共通処理を利用。
- [ ] 3-13 `--same-owner` 既定値の調整とテスト。

---
## 4. 出力・UX（ロードマップ §1.6）
- [ ] 4-1 `-v/--verbose`
  - 実装: create/extract/list/append ごとに対象ファイルを stdout に表示。ロガーとの競合を整理。
  - テスト: `-tvf` などで bsdtar の出力と diff。
- [ ] 4-2 `--totals`
  - 実装: 終了時にファイル数・バイト数などを表示。
  - テスト: `--totals` の出力を比較。
- [ ] 4-3 `-O/--to-stdout`
  - 実装: 抽出モードで stdout へストリーミングする関数を追加。
  - テスト: `-xOf archive file` の結果を `cmp` で比較。
- [ ] 4-4 `-S`
  - 実装: スパースファイルの抽出/作成対応。
  - テスト: `punch hole` を使ったファイルで比較。
- [ ] 4-5 `-U/--unlink-first`
  - 実装: 抽出前にターゲットを unlink。
  - テスト: ハードリンク/パーミッション付きファイルで比較。
- [ ] 4-6 `-P/--absolute-paths`
  - 実装: 安全チェックの無効化フラグを追加し、`allow_unsafe_links` を連携。
  - テスト: 絶対パス/.. 含むエントリの挙動比較。
- [ ] 4-7 `--safe-writes` / `--no-safe-writes`
  - 実装: 一時ファイル → rename のモード切替。
  - テスト: 障害発生時のロールバックを検証。
- [ ] 4-8 `--ignore-zeros` (テスト再確認)／`--no-safe-writes`
  - 実施: 2-22, 4-7 と連携したテストを追加。
- [ ] 4-9 `--version`
  - 実装確認: `pna experimental stdio --version` で互換情報を表示。

---
## 5. テスト強化（ロードマップ §2, §4, §6）
- [ ] 5-1 ゴールデン比較ハーネス実装 (`tests/bats/lib/shared/compare_with_bsdtar.bash`)。
- [ ] 5-2 既存 Bats テストをゴールデン比較化。
- [ ] 5-3 libarchive tar テストスイート実行ラッパ (`tests/bats/libarchive/run_libarchive_suite.bash`) を作成し、全ケースを通過。
- [ ] 5-4 Clap パーサ単体テスト (`cli/tests/parser.rs`) を追加し、成功パターン・失敗パターンを網羅。
- [ ] 5-5 CI 更新: macOS/Linux でゴールデン比較、libarchive テスト、`cargo test` を実行。
- [ ] 5-6 `--unstable` フラグ無しでの最終互換テストを CI に追加。

---
## 6. ドキュメント（ロードマップ §5）
- [ ] 6-1 `README.md` に互換モード解説・対応オプション表・制限事項 (フォーマット切替不可、ハイフン無し非対応など) を掲載。
- [ ] 6-2 `cli/README.md` と `--help` メッセージを更新し、新規オプションを網羅。
- [ ] 6-3 リリースノートに互換完了条件・テストカバレッジを明記。
- [ ] 6-4 Issue/Discussion 用テンプレートを整備（ユーザーフィードバック収集）。

---
## 7. 総合検証
- [ ] 7-1 手動シナリオ: create/extract/list/update/delete/append-to を `bsdtar` と比較。
- [ ] 7-2 圧縮 (gzip/bzip2/xz/zstd/lz4/lzma/lzop/lrzip) の往復確認。
- [ ] 7-3 ACL/xattr/fflags/mac metadata の round-trip.
- [ ] 7-4 `git status` で不要ファイルが無いことを確認し、テストアーティファクトを削除。
- [ ] 7-5 CI が全て緑であることを確認。

---
## 進捗チェックリスト
- [x] 0.準備完了 — 0-1〜0-4 まで完了済み。
- [x] 1.モード互換 — 1-1〜1-6 を実装済み（`@archive` は stdout 出力時のみ未対応という既知制限あり）。
- [ ] 2.入出力・探索 — 残タスク: 2-4 `--chroot`, 2-11 `--include/--exclude` の互換 glob, 2-12 `list` での `--exclude-vcs`, 2-18 `-L/-h` など。
- [ ] 3.時間・所有権・メタデータ — 残タスク: 3-5 `-o`, 3-9〜3-13 の属性トグル・デフォルト確認。
- [ ] 4.出力・UX — 残タスク: 4-1〜4-9 全體。
- [ ] 5.テスト体制 — ゴールデン比較/CI 強化未着手。
- [ ] 6.ドキュメント — README/CLI help/リリースノート更新待ち。
- [ ] 7.総合検証 — 手動比較・圧縮往復などは未実施。
