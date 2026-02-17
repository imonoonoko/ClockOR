# ClockOR

Windows フルスクリーンゲーム向けの時計オーバーレイアプリケーションです。
ゲームプレイ中でも現在時刻を確認できます。

## 特徴

- フルスクリーンゲームの上に時計を常時表示
- ホットキーで表示/非表示を切り替え
- 画面4隅から表示位置を選択
- 24時間 / 12時間表示
- 秒表示の有無を選択
- フォントサイズ（Small / Medium / Large）
- 透明度調整（25〜100%）
- システムトレイ常駐
- Windows起動時の自動起動

## インストール

### リリースからダウンロード

[Releases](https://github.com/imonoonoko/ClockOR/releases) ページから最新の `clockor.exe` をダウンロードしてください。

### ソースからビルド

```bash
git clone https://github.com/imonoonoko/ClockOR.git
cd ClockOR
cargo build --release
```

ビルド成果物は `target/release/clockor.exe` に出力されます。

## 使い方

1. `clockor.exe` を起動するとシステムトレイにアイコンが表示されます
2. ホットキー（デフォルト: `Ctrl+F12`）で時計オーバーレイの表示/非表示を切り替えます
3. トレイアイコンを右クリック → **Settings** で設定画面を開きます

## 設定ファイル

設定は以下の場所に保存されます:

```
%APPDATA%\ClockOR\config.toml
```

## ホットキー

デフォルトのホットキーは `Ctrl+F12` です。
設定画面から修飾キー（Ctrl / Alt / Shift の組み合わせ）とファンクションキー（F1〜F12）を選択できます。

## ライセンス

[MIT License](LICENSE)
