# COBOL_to — 開発メモ / Development notes

**日本語 → English**

## 日本語

### 役割
COBOL(各方言・各版・固定/自由形式)を自動判定し、Python(FastAPI/MFastAPI)または Rust(Poem/RPoem)のプロジェクトへ変換するツール。作業場所は `F:\COBOL_to`、GitHub は `aon-co-jp/COBOL_to`(公開、MIT)。`COBOL2` → `COBOL-to` → `COBOL_to` と改名した経緯があるが、**履歴は新規**(旧名は残っていない)。

### 構成
- `src/detect.rs` 方言・規格・書式の判定(キーワードと桁位置の根拠つき)
- `src/convert.rs` 字句解析 → 共通 IR(`Program`/`Var`/`Stmt`)→ `to_rust` / `to_python`。単体テストは `hello_roundtrip`
- `src/main.rs` フォルダ選択(`rfd`)、変換先の選択、`.cbl/.cob/.cobol/.pco` の再帰走査、雛形生成、`COBOL_to-REPORT.md`
- `installer/` Windows(`cobol_to.iss`)、Linux(`build-deb.sh`)、macOS(`build-pkg.sh`)。`.github/workflows/release.yml` が全 OS を作って試験
- `NOTIFY.md` / `notify.example.toml` 更新通知(version-control の `[notify] email_to` に 2 件)

### 方針(守ること)
- **REST は作らない**。API は Connect(`POST /cobol_to.Program/Run`)。JSON は RS-JSON に寄せる方針(現状は未使用の小さなコード)。
- **未対応の構文は黙って落とさない**(`TODO(COBOL_to)`)。生成物の依存は**固定しない**。
- RPoem 向けには Bootstrap・TypeScript・Next.js・Node.js を**使わない**(RPoem は Rust→WASM)。
- Windows 配布は `.exe`(Inno Setup)。PowerShell スクリプトを利用者向けの入口にしない。
- 走査の起点(`root`/`owner`)は各ユーザーが指定する。`F:/` や `aon-co-jp` は作者の例。
- メールアドレスは `config.toml`(git 管理外)に置き、リポジトリへ入れない。
- 文書は日本語と英語(通知文書のみ日英独露。ドイツ語・ロシア語は AI 翻訳でネイティブ検証前)。

### 環境の落とし穴
- この PC の Bash では `cargo` が PATH に無い。**PowerShell で `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"`** を前置きする(WSL の Ubuntu には cargo あり)。
- **`cargo test` では `target\debug\cobol_to.exe` は更新されない**。変更後の実行確認は `cargo build` してから。
- Bash の heredoc に `'` を含む長い内容を入れると構文エラーになる。ファイルは Write ツールで作る。
- PowerShell 5.1 は `git push` の進行表示を赤い「エラー」で出す(終了コードと `git status -sb` で判断)。日本語を含むファイルを PowerShell で書かない(文字化け)。
- Windows で `Select-String` を一時出力フォルダのディレクトリに向けるとアクセス拒否になる(ファイルを個別に指定する)。

### 状態(2026-11-18)
- 手元で確認済み: テスト成功、Windows x64 のインストール/実行/アンインストール、Linux amd64 の `dpkg -i`/実行/`-r`(WSL)。
- `v0.1.0` タグを push 済みで、リリース CI を実行中(結果は GitHub Actions の `release`)。Windows ARM64・Linux arm64・macOS は CI が初めての確認。
- version-control は `root = "F:/"` なので `F:\COBOL_to` を自動で走査する(動作確認は走査が長く未了)。通知メールは `email_to` の 2 件(手元と VPS の両方の `config.toml`)。VPS で `version-control.service` を実行して `e-mail: sent` を確認、受信箱への到達は未確認、LINE は未設定。

## English

### Role
A tool that auto-detects COBOL (dialects, standards, fixed/free format) and converts to Python (FastAPI/MFastAPI) or Rust (Poem/RPoem) projects. Working copy `F:\COBOL_to`; GitHub `aon-co-jp/COBOL_to` (public, MIT). It was renamed `COBOL2` → `COBOL-to` → `COBOL_to`, but the **history is fresh** (no old names remain).

### Layout
- `src/detect.rs` dialect/standard/format detection with evidence
- `src/convert.rs` tokenizer → shared IR (`Program`/`Var`/`Stmt`) → `to_rust` / `to_python`; unit test `hello_roundtrip`
- `src/main.rs` folder picker (`rfd`), target choice, recursive scan of `.cbl/.cob/.cobol/.pco`, scaffolding, `COBOL_to-REPORT.md`
- `installer/` Windows (`cobol_to.iss`), Linux (`build-deb.sh`), macOS (`build-pkg.sh`); `.github/workflows/release.yml` builds and tests every OS
- `NOTIFY.md` / `notify.example.toml` update notifications (two addresses in version-control's `[notify] email_to`)

### Rules
- **No REST.** The API is Connect (`POST /cobol_to.Program/Run`). JSON should move to RS-JSON (currently not used by this small code).
- **Never drop unsupported syntax silently** (`TODO(COBOL_to)`). Do **not pin** dependencies of generated projects.
- No Bootstrap/TypeScript/Next.js/Node.js for the RPoem target (RPoem is Rust→WASM).
- Windows ships as `.exe` (Inno Setup); PowerShell scripts are not a user entry point.
- The scan start points (`root`/`owner`) are specified by each user; `F:/` and `aon-co-jp` are the author's examples.
- E-mail addresses live in `config.toml` (git-ignored), never in the repository.
- Docs are Japanese and English (only the notification doc is JA/EN/DE/RU; German and Russian are AI translations, not native-checked).

### Environment pitfalls
- `cargo` is not on PATH in this PC's Bash. Use **PowerShell with `$env:PATH="$env:USERPROFILE\.cargo\bin;$env:PATH"`** (WSL Ubuntu has cargo).
- **`cargo test` does not refresh `target\debug\cobol_to.exe`**; run `cargo build` before checking behaviour.
- Long Bash heredocs containing `'` fail to parse; create files with the Write tool.
- PowerShell 5.1 shows `git push` progress as a red "error" (judge by exit code and `git status -sb`); do not write Japanese files from PowerShell (mojibake).
- `Select-String` on a directory in a temp output folder gives access-denied (name files individually).

### Status (2026-11-18)
- Verified locally: tests pass, Windows x64 install/run/uninstall, Linux amd64 `dpkg -i`/run/`-r` (WSL).
- Tag `v0.1.0` is pushed and the release CI is running (result: GitHub Actions `release`). Windows ARM64, Linux arm64 and macOS are first verified by CI.
- version-control uses `root = "F:/"`, so `F:\COBOL_to` is scanned automatically (a live check was not finished; the scan is long). Notification mail goes to the two `email_to` addresses (both the local and the VPS `config.toml`). Running `version-control.service` on the VPS logged `e-mail: sent`; delivery to the inboxes is unconfirmed; LINE is not configured.
