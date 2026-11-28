![Cedarflake Ame](docs/images/ame.png)

<p align="center"><a href="./README.md">English</a> | <a href="./README.zh-Hans.md">简体中文</a> | 日本語</p>

Ame は、写真や画像の日常的な閲覧・整理に使える Windows 向けローカル画像ライブラリアプリです。画像の閲覧、自然言語検索、自動分類、重複・類似画像の整理、ファイル管理の機能を備えています。[Apache-2.0 ライセンス](LICENSE)で公開されている、無料のオープンソースソフトウェアです。

![Ame のギャラリー、画像ビューアー、検索、アルバム、重複グループ、ファイル整理の画面](docs/images/ame-showcase.webp)

## ダウンロード

Ame は現在 **Alpha 版**です。主要機能は実装済みで、Beta 版に向けて性能と安定性の改善を進めています。[リリースページ](https://github.com/Cedarflake/Cedarflake-Ame/releases)から **Windows 11 x64** 向けのパッケージをダウンロードできます。

- **MSI インストーラー**には画像解析モデルとランタイムが含まれています。ライブラリをバックグラウンドで更新する Windows サービスもインストールします。インストールには管理者権限が必要です。
- **ポータブル版 ZIP** はインストール不要で利用できます。画像解析モデルは初めて使うときにダウンロードされ、Windows サービスはインストールされません。

[Windows 11 向けプレビュー版](https://github.com/Cedarflake/Cedarflake-Ame/actions/workflows/quality_windows_arm64.yml)には、x64 版と ARM64 版の MSI、および ARM64 用のポータブル版 ZIP があります。Arm 搭載 PC で x64 版 MSI から ARM64 版 MSI に切り替える場合は、既存の MSI 版をアンインストールしてから再インストールする必要があります。

アプリ内での更新に対応し、自動確認と自動ダウンロードを選べます。更新機能のない旧版は手動でのアップグレードが必要です。リリース版とプレビュー版のパッケージは未署名です。変更点や既知の制限は[リリースノート](docs/releases/README.md)に記載しています。

## 主な機能

- **ギャラリー表示**：縦横比を保った写真の一覧、正方形グリッド、タイムラインに対応。複数のフォルダーやドライブの画像を一つのライブラリで閲覧できます。
- **自動同期**：ファイルの追加・変更・削除をバックグラウンドでライブラリに反映し、アプリの終了中に発生した変更にも追従します。
- **検索**：ファイル名・パス、自然言語、画像を使った検索に対応。ファイル形式、日付、画像の寸法、ファイルサイズ、カメラ情報による絞り込みを組み合わせられます。
- **スマートアルバム**：SigLIP 2 を使って PC 上で画像を自動分類します。対応デバイスでは GPU アクセラレーションが既定で有効になっています。
- **お気に入りとアルバム**：お気に入りや仮想アルバムで画像を整理し、アルバムのアイコンを選べます。整理の進行状況を保存し、続きから再開できます。
- **重複・類似画像の整理**：内容が一致する画像と見た目が似た画像をそれぞれグループ化し、比較や不要な画像の整理に対応します。類似度の基準も調整できます。
- **ファイル管理**：Windows の機能を使ったコピー、移動、名前の変更、削除に対応。操作履歴と、中断した操作の内容に応じた復旧機能を備えています。
- **画像プレビュー**：拡大・縮小、表示の回転、2 枚の画像の比較、サムネイルによる画像の切り替えに対応しています。

## ソースから実行する

Ame は開発中です。Windows 11 x64 でビルドするには、Flutter **3.50.0**、[rust-toolchain.toml](rust-toolchain.toml) で指定された Rust ツールチェーン、Visual Studio 2022 が必要です。Visual Studio では「C++ によるデスクトップ開発」ワークロード、Windows SDK、CMake ツールをインストールしてください。Windows on Arm でビルドする場合は、[ARM64 のビルド手順](docs/acceptance/quality/commands.md#build-on-windows-arm64)を参照してください。

```powershell
flutter pub get
flutter run -d windows
```

画面には Flutter と Material 3 を使用しています。Rust がカタログ、ファイルの変更追跡、プレビュー、画像解析、ファイル操作を担当し、ライブラリのデータは SQLite に保存されます。

詳しくは[ロードマップ](docs/roadmap.md)、[開発ガイド](docs/development/repository-layout.md)、[ビルドとテストのコマンド](docs/acceptance/quality-gates.md)、[アーキテクチャの解説](docs/architecture/README.md)を参照してください。

## ライセンス

Copyright 2026 Cedarflake Ame contributors.

特記がある場合を除き、Cedarflake Ame には [Apache License, Version 2.0](LICENSE)（`Apache-2.0`）が適用されます。サードパーティのコード、依存ライブラリ、モデルの重み、リソースには、それぞれのライセンスと表記が適用されます。

利用条件は[エンドユーザー使用許諾契約](EULA.md)（英語）を参照してください。
情報の取り扱いは[プライバシーに関する声明](PRIVACY.md)（英語）を参照してください。
