# COBOL_to

COBOL の方言・版・書式を自動判定し、Rust/RPoem/Python 系スタックへ変換するツール。/ Detects the COBOL dialect, standard and source format automatically and converts to Rust/RPoem/Python stacks.

## 使い方 / Usage
```
cobol_to                                  # フォルダ選択ダイアログが開く / opens a folder picker
cobol_to --src <dir> --target <fastapi|mfastapi|rust-poem|rpoem> --out <dir>
```
- 判定: IBM Enterprise / Micro Focus / GnuCOBOL / ACUCOBOL / NetCOBOL / 日立、COBOL-68〜2014、固定/自由形式。結果は出力先の `COBOL_to-REPORT.md`。
- API: Connect 形式 `POST /cobol_to.Program/Run`(REST は使わない)。
- 変換できる構文は現状 `PROGRAM-ID`/`01・77`/`DISPLAY`/`MOVE`/`ADD`/`STOP RUN` のみ。未対応の文は `TODO(COBOL_to)` として出力。EBCDIC は未変換。
- 更新通知: [NOTIFY.md](NOTIFY.md)(日英独露)。

License: MIT

## インストーラー / Installers
| OS | 形式 / Format | 作り方 / Build |
|---|---|---|
| Windows x64 / ARM64 | `COBOL_to-Setup-x64.exe` / `-arm64.exe` (Inno Setup。ユーザー単位、PATH追加は任意) | `installer/cobol_to.iss` |
| Linux amd64 / arm64 | `cobol-to_<版>_<arch>.deb`、`.tar.gz` | `installer/build-deb.sh` |
| macOS arm64 / x64 | `COBOL_to-<版>-macos-<arch>.pkg`(未署名) | `installer/build-pkg.sh` |

タグ `v*` の push で `.github/workflows/release.yml` が全OS分を作り、各OSで install→実行→uninstall の通し試験をしてから Release に添付します。
Tagging `v*` builds every installer, smoke-tests install → run → uninstall on each OS, then attaches them to the Release.

検証状況 / Status: Windows x64(install/実行/uninstall)と Linux amd64(WSLで dpkg -i/実行/-r)は手元で確認済み。Windows ARM64・Linux arm64・macOS は CI 初回実行まで未確認、macOS の署名・公証は未対応(Apple のシークレットが必要)。
