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
