# Baker link. Env

**[English](README.md) | [日本語](README.ja.md)**

Baker link.シリーズの組込みRust開発を支援するアプリです。
VS CodeのDev Containerで編集・ビルドし、PC上のprobe-rsを通して実機をデバッグします。
通常の利用では、PCにRustの開発ツール一式を入れる必要はありません。

## はじめに

1. [Releases](https://github.com/Baker-link-Lab/baker-link-env/releases)からアプリをインストールします。
2. [VS Code](#vscode)・[Dev Containers](#dev-containers)・[Docker](#docker-cli)を準備します。
3. アプリを起動します。**環境診断**で問題が出たら、**解決手順を読む**で対処し、**再診断**します。
4. **プロジェクト作成**に名前を入力し、**作成**で保存先を選びます。
   VS Codeのコンテナ起動とBaker Link拡張のインストールが終わるまで待ちます。
5. ボードとデバッグプローブを接続し、PC側のアプリで**実行**を押します。
   VS Codeで **F5** を押すと、ビルド・ELF転送・デバッグを開始します。

上部の **English / 日本語** でアプリとヘルプの言語を切り替えます。選択は次回起動時にも引き継がれます。
コマンド・パス・デバイス名・外部ツールの出力は原文、OS標準ダイアログはOSの言語で表示します。
設定をリセットすると英語に戻ります。

<a id="environment-setup"></a>
## 開発環境の準備

「ホスト」は、このアプリを起動しているPCのことです。
起動時の環境診断は、ホストのツールとDockerの対応状況を確認します。
インストールや設定変更は行いません。各コマンドのタイムアウトは15秒です。

| 診断結果 | 次にすること |
|---|---|
| 正常 | この項目の確認は成功しています |
| 対応が必要 | **解決手順を読む**で対処して、**再診断** |
| 未確認 | 先に前提となる項目を直す |

最初に問題が出た項目から確認してください：
[VS Code](#vscode)・[Dev Containers](#dev-containers)・
[Docker CLI](#docker-cli)・[Dockerエンジン](#docker-engine)・[ホスト接続](#host-networking)。

**← ホームへ戻る**で戻り、通知は **×** で閉じられます。
解決しない場合は**詳細**を確認し、**診断結果をコピー**して相談してください。
共有前に個人のパスやホスト名を確認してください。オンラインヘルプにはインターネット接続が必要です。
診断の正常は、プロジェクト・コンテナ接続・USBやMCUの動作まで保証するものではありません。

<a id="vscode"></a>
### VS Codeを準備する

1. [VS Code](https://code.visualstudio.com/download)をインストールし、一度起動します。
2. `code` コマンドを有効にします。
   - **Windows:** インストーラーで **PATHへの追加（Add to PATH）** を選択。
   - **macOS:** `/Applications` または `~/Applications` に配置。別の場所ならコマンドパレットで
     **Shell Command: Install 'code' command in PATH** を実行。
   - **Debian:** [公式Linux手順](https://code.visualstudio.com/docs/setup/linux)に従って導入。
3. ホストの新しいターミナルで `code --version` を確認します。
   PATHを変更した場合はアプリを完全に終了して起動し直し、再診断します。

失敗する場合は、診断の詳細で権限エラーやタイムアウトを確認してください。

<a id="dev-containers"></a>
### Dev Containersを入れる

1. コンテナに接続していない、ホスト側のVS Codeを開きます。
2. 拡張機能を開きます（`Ctrl+Shift+X`、macOSは `Cmd+Shift+X`）。
3. Microsoftの [Dev Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)
   （`ms-vscode-remote.remote-containers`）をインストールし、再診断します。

`code` コマンドが使うVS Codeの標準プロファイルに入れてください。
拡張一覧の取得に失敗する場合は、先にVS Codeの診断項目を直します。

<a id="docker-cli"></a>
### Dockerを準備する

1. [Rancher Desktop](https://docs.rancherdesktop.io/getting-started/installation/)を入れ、
   エンジンを **dockerd (moby)** にします（`containerd` のみでは利用できません）。
   Docker Desktop、Debian上の通常の[Docker Engine](https://docs.docker.com/engine/install/debian/)も対応します。
2. Windowsではランタイムの案内に従ってWSL 2を準備します。
   Docker Desktopは **Linux containers** を使います。
3. ランタイムのCLI・PATH設定に従い、新しいターミナルで `docker --version` を確認します。
   アプリを完全に終了して起動し直し、再診断します。

迷う場合は1つのランタイムを使ってください。CLIが動いてもエンジンが起動しているとは限りません。

<a id="docker-engine"></a>
### Dockerに接続できないとき

1. Dockerを起動して準備完了を待ち、ホストのターミナルで `docker info` を確認します。
2. 失敗する場合は `docker context ls` で接続先（`*`）を確認します。
   必要なら `docker context use NAME` で、自分のランタイムの接続先を選びます。
3. Debianの権限エラーは[公式の導入後手順](https://docs.docker.com/engine/install/linux-postinstall/)を確認します。
   Dockerグループはホストを操作する強い権限を持つため、注意事項を読んでください。
4. 再診断します。出力形式のエラーやタイムアウトが続く場合は、Dockerの更新・修復を確認します。

<a id="host-networking"></a>
### ホスト接続に対応する環境

| Docker環境 | ホストへの接続 |
|---|---|
| Rancher Desktop / Docker Desktop | 標準の `host.docker.internal` 名前解決 |
| Debian/Linux上のローカル・rootful Docker Engine | `host.docker.internal:host-gateway` |

Windowsコンテナ・rootlessの通常Docker・リモートの通常Dockerは自動設定の対象外です。
IPの推測やファイアウォール無効化ではなく、`docker context ls` と `docker info` で環境を確認してください。

アプリ経由でテンプレートのプロジェクトを開くと `.docker/compose.host.yaml` を更新します。
OSやランタイム変更後はアプリから開き直し、VS Codeで **Dev Containers: Rebuild Container** を実行します。
再作成はコンテナ内のプロセスを停止するため、先に作業を保存してください。
Desktop環境の基本Composeに無条件の `host-gateway` を追加しないでください。
手動で開く場合は[テンプレートの手順](template/bakerlink_tutorial_template/README.md)を参照してください。

## デバッグ・接続の確認

- 標準の待受IPは `0.0.0.0`、DAPポートは `50001`、アップロードは `50002` です。先にPC側で**実行**を押します。
- ポートを変えたら `.vscode/launch.json` の `bakerLink.dapServer` と `bakerLink.uploadUrl` も変更します。
- 接続確認はVS Codeの **Baker Link: Check Host Connection**、エラー詳細は **Baker Link Env** 出力チャンネルで確認します。
- リリース・サンプルのデバッグでは、ビルドタスク・`bakerLink.elf`・成果物のプロファイルを合わせます。
- テンプレートは `haltAfterReset: false` で、書き込み・デバッガ設定後に自動実行します。
  RTTはファームウェアの初期化後に専用ターミナルへ表示されます。
  `haltAfterReset: true` の場合は初期停止後に **続行（F5）** が必要です。
- ELFアップロード成功はPC側キャッシュへの保存完了を示し、MCUへの書き込み成功とは異なります。

**安全上の注意:** DAPとELFアップロードは認証がありません。`0.0.0.0` は全インターフェースで待受するため、
信頼できるネットワークだけにファイアウォールでアクセスを制限してください。
通常のコンテナ接続では `127.0.0.1` のみの待受は使えません。

<details>
<summary>既存プロジェクト・開発者向けの情報</summary>

テンプレートはコンテナ内にBaker Link拡張を自動導入します。既存コンテナは再作成または手動導入が必要です。
旧 `.bakerlink` からの移行、ソースビルド、拡張機能の開発は[英語README](README.md#debugging-and-connection-problems)を参照してください。

</details>

### macOSで「壊れているため開けません」と出る場合

公式配布元から取得したアプリに限り、[英語READMEの手順](README.md#macos-damaged-app-error)で隔離属性を解除できます。

## ライセンス

Copyright (c) 2024 Baker-Tanaka. [MIT](LICENSE)。
