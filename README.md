# ClockOR

Windows フルスクリーンゲーム向けの時計オーバーレイアプリケーションです。
ゲームプレイ中でも現在時刻を確認できます。

## 特徴

- フルスクリーンゲームの上に時計を常時表示
- ホットキーで表示/非表示を切り替え（トレイ左クリックでも切替可能）
- 画面4隅から表示位置を選択
- 24時間 / 12時間表示、秒表示の有無を選択
- フォントサイズ自由設定（10〜60px）
- テキスト色・縁取り/影色のカスタマイズ（色ピッカー）
- テキストスタイル選択（なし / 縁取り / 影）
- 透明度調整（25〜100%）
- High DPI 対応（Per-Monitor V2）
- 多重起動防止
- システムトレイ常駐
- Windows 起動時の自動起動

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
3. トレイアイコンを左クリックでも表示/非表示を切り替えられます
4. トレイアイコンを右クリック → **Settings** で設定画面を開きます

## 設定

### 設定画面

| セクション | 項目 | 説明 |
|-----------|------|------|
| **Display** | Position | 画面のどの角に時計を表示するか |
| | Time Format | 24時間 / 12時間表示 |
| | Show seconds | 秒の表示/非表示 |
| **Appearance** | Font Size | テキストのピクセル高さ（10〜60） |
| | Text Style | None / Outline / Shadow |
| | Text Color | テキストの色 |
| | Outline/Shadow Color | 縁取りまたは影の色 |
| | Opacity | オーバーレイの透明度 |
| **System** | Hotkey | 表示/非表示を切り替えるキー |
| | Start with Windows | Windows 起動時に自動起動 |

「Reset to Defaults」ボタンで全設定を初期値に戻せます（Apply で確定するまで保存されません）。

### 設定ファイル

設定は以下の場所に保存されます:

```
%APPDATA%\ClockOR\config.toml
```

## ホットキー

デフォルトのホットキーは `Ctrl+F12` です。
設定画面から修飾キー（Ctrl / Alt / Shift の組み合わせ）とファンクションキー（F1〜F12）を選択できます。

## ライセンス

[MIT License](LICENSE)
