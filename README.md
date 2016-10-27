# レポートHTML錬成機
Markdown からﾌｧｯｷﾝ Moodle 向け HTML を生成するやつ。

# 必要環境
* コンパイルするなら: Rust, Cargo
* 実行時に必要: Pandoc

# ダウンロード
[いちおうある](https://github.com/azyobuzin/reportyatsu/releases/tag/v0.2.0)

# 使い方
```
reportyatsu 0.2.0

USAGE:
    reportyatsu [OPTIONS] <INPUT>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
        --highlight-style <STYLE>
            Pandoc の --highlight-style [values: pygments, kate, monochrome, espresso,
            zenburn, haddock, tango]
        --stylesheet <CSSFILE>       カスタム CSS

ARGS:
    <INPUT>...    入力する Markdown ファイル
```
