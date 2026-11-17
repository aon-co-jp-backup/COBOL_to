# COBOL_to — お引越し・再開 / Porting and resume

**日本語 → English**

## 日本語

### 別の PC・別アカウントで再開する手順
1. `git clone https://github.com/aon-co-jp/COBOL_to` して `cd COBOL_to`。
2. Rust(最新の stable)を入れる。`cargo build --release` と `cargo test`。
3. Windows のインストーラーを作るなら Inno Setup 6 を入れ、`ISCC /DAppVersion=0.1.0 /DArch=x64 /DExeDir=..\target\release installer\cobol_to.iss`(出力は `dist\`)。Linux の `.deb` は `installer/build-deb.sh`、macOS の `.pkg` は macOS 上で `installer/build-pkg.sh`。
4. 動作確認: `cobol_to --src tests/samples --target rust-poem --out <出力先>`。`src/programs/hello.rs` ができ、`COBOL_to-REPORT.md` に判定が出る。
5. 更新通知は version-control の `config.toml`(git 管理外)の `[notify] email_to` に自分のメールを 2 件書く。`root` も自分のドライブ/フォルダにする。秘密(トークン等)はリポジトリに入れない。

### 未了(優先順)
1. **リリース CI の結果確認**(タグ `v0.1.0`)。失敗したジョブ(特に Windows ARM64・Linux arm64・macOS の通し試験)を直して、タグを付け直さず `v0.1.1` で出す。
2. **変換できる構文の拡充**: `IF`/`EVALUATE`/`PERFORM`(`UNTIL`・`VARYING`)/`COMPUTE`/`SUBTRACT`/`MULTIPLY`/`DIVIDE`/`STRING`/`INSPECT`/`COPY`・`REPLACING`/ファイル I/O/`CALL`/`EXEC SQL`・`EXEC CICS`、数値の小数・符号・`COMP-3`、CONTINUATION 行。テストは `tests/samples` に実プログラムを足して追加。
3. **生成物の実サーバー接続**: FastAPI/MFastAPI/Poem/RPoem に `POST /cobol_to.Program/Run`(Connect)を実装。Next.js(TypeScript+Bootstrap)の画面の自動生成。RPoem は Rust→WASM の画面。
4. **最新版への自動追従**: 生成物の依存は固定していない。`cargo add` と PyPI/npm の最新取得、version-control の登録。
5. **通知の到達確認**: VPS で `e-mail: sent` は確認したが、`norukia.jp@gmail.com` と `cloud9slack@gmail.com` の受信箱への到達は未確認。LINE は未設定。
6. **GitHub の owner 配下を起点にした走査**: version-control には未実装(`root` のみ)。
7. **macOS の署名・公証**(Apple のシークレットが必要)、EBCDIC の自動変換、Linux の `.rpm`/AppImage。
8. **version-control による COBOL_to の実走査確認**(`root = "F:/"` で拾われるはず。走査が長いので `--dry-run --no-fetch --no-api` を時間に余裕のあるときに)。

### 注意
- この PC の Bash に `cargo` は無い(PowerShell で PATH を前置き)。`cargo test` は debug の exe を更新しない。
- 履歴は新規。旧名(`COBOL2`/`COBOL-to`)のリポジトリは作っていない。

### 再開用メッセージ(そのまま貼り付け)
> `F:\COBOL_to`(GitHub: aon-co-jp/COBOL_to)を再開してください。まず `README.md`・`CLAUDE.md`・`PORTING.md` を読むこと。COBOL の方言・版・書式を自動判定し、FastAPI / MFastAPI / Rust+Poem / Rust+RPoem(RPoem は Bootstrap・TypeScript・Next.js・Node.js なし)へ変換するツールで、API は REST ではなく Connect(`POST /cobol_to.Program/Run`)。v0.1.0 のタグを push 済みで、リリース CI の結果を最初に確認し、失敗があれば直して v0.1.1 で出してください。次に、`PORTING.md` の未了の優先順(構文の拡充 → 実サーバー接続 → 最新版への自動追従)を進めてください。注意: Bash に cargo は無いので PowerShell で PATH を前置きする、`cargo test` は debug の exe を更新しない、未対応の構文は黙って落とさず `TODO(COBOL_to)` にする、メールアドレスやトークンはリポジトリに入れない、`git pull` 前に `git status -sb` で分岐を確認する。

## English

### Resume on another PC or account
1. `git clone https://github.com/aon-co-jp/COBOL_to` and `cd COBOL_to`.
2. Install the latest stable Rust. `cargo build --release` and `cargo test`.
3. For the Windows installer install Inno Setup 6 and run `ISCC /DAppVersion=0.1.0 /DArch=x64 /DExeDir=..\target\release installer\cobol_to.iss` (output in `dist\`). The Linux `.deb` is `installer/build-deb.sh`; the macOS `.pkg` is `installer/build-pkg.sh` on macOS.
4. Check: `cobol_to --src tests/samples --target rust-poem --out <out>` creates `src/programs/hello.rs` and the detection in `COBOL_to-REPORT.md`.
5. For update notifications put your own two addresses into `[notify] email_to` in version-control's `config.toml` (git-ignored) and set `root` to your drive/folder. Never commit secrets.

### Open items (priority order)
1. **Check the release CI result** (tag `v0.1.0`). Fix failing jobs (especially the Windows ARM64, Linux arm64 and macOS smoke tests) and ship as `v0.1.1` rather than moving the tag.
2. **Widen the supported syntax**: `IF`/`EVALUATE`/`PERFORM` (`UNTIL`, `VARYING`)/`COMPUTE`/`SUBTRACT`/`MULTIPLY`/`DIVIDE`/`STRING`/`INSPECT`/`COPY`+`REPLACING`/file I/O/`CALL`/`EXEC SQL`+`EXEC CICS`, decimals/signs/`COMP-3`, continuation lines. Add real programs to `tests/samples` as tests.
3. **Wire the output into real servers**: implement `POST /cobol_to.Program/Run` (Connect) in FastAPI/MFastAPI/Poem/RPoem; generate the Next.js (TypeScript + Bootstrap) UI; a Rust→WASM UI for RPoem.
4. **Automatic latest versions**: generated dependencies are not pinned. Use `cargo add` and the PyPI/npm latest lookups; register in version-control.
5. **Confirm delivery of notifications**: `e-mail: sent` was logged on the VPS, but arrival in the inboxes of `norukia.jp@gmail.com` and `cloud9slack@gmail.com` is unconfirmed; LINE is not configured.
6. **Scanning from a GitHub owner**: not implemented in version-control (only `root`).
7. **macOS signing/notarization** (Apple secrets needed), EBCDIC auto-conversion, Linux `.rpm`/AppImage.
8. **Live version-control scan of COBOL_to** (should be picked up via `root = "F:/"`; the scan is long, so run `--dry-run --no-fetch --no-api` when there is time).

### Notes
- No `cargo` in this PC's Bash (prepend PATH in PowerShell). `cargo test` does not refresh the debug exe.
- History is fresh. No repositories were created under the old names (`COBOL2`/`COBOL-to`).

### Resume message (paste as is)
> Please resume `F:\COBOL_to` (GitHub: aon-co-jp/COBOL_to). Read `README.md`, `CLAUDE.md` and `PORTING.md` first. It auto-detects the COBOL dialect, standard and source format and converts to FastAPI / MFastAPI / Rust+Poem / Rust+RPoem (RPoem without Bootstrap, TypeScript, Next.js or Node.js); the API is Connect (`POST /cobol_to.Program/Run`), not REST. The v0.1.0 tag is pushed: first check the release CI result and, if anything failed, fix it and ship v0.1.1. Then work through the open items in `PORTING.md` in order (syntax coverage → real server wiring → automatic latest versions). Cautions: Bash has no cargo, so prepend PATH in PowerShell; `cargo test` does not refresh the debug exe; never drop unsupported syntax silently, emit `TODO(COBOL_to)`; keep e-mail addresses and tokens out of the repository; check `git status -sb` for divergence before any `git pull`.
