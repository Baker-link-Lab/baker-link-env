# Baker link. Env — 日本語Help

[English Help](README.md) | [日本語Help](README.ja.md)

Baker link. Envは、Baker link.シリーズの組込みRust開発を支援するデスクトップアプリです。
VS CodeのDev Containerでコードを編集・ビルドし、ホスト上のprobe-rsを通して実機をデバッグできます。
通常の利用では、ホストにRustの開発ツール一式をインストールする必要はありません。

## doctor-start-here

### はじめての環境セットアップ

準備するものは次の3つです。「ホスト」とは、Baker Link Envを起動しているWindows・macOS・Debianのコンピューターのことです。

| 必要なもの | 役割 |
|---|---|
| VS Code | コードを編集するアプリ |
| Dev Containers | VS Codeから開発用コンテナを開くための拡張機能 |
| Docker | 開発用コンテナを動かす仕組み。初めてならRancher Desktopから始めるのがおすすめです |

**Doctorは起動時に自動実行されます。問題があるとDoctor画面と通知を表示します。**
診断だけを行い、ソフトのインストール、Dockerの起動、設定やファイアウォールの変更は行いません。

1. Doctorの **Needs attention（対応が必要）** と表示された項目を確認します。
2. 上部の言語選択を **Help: 日本語** に変更し、項目の **解決手順を読む（日本語Help）** を押します。
   このHelpの該当箇所がブラウザーで開きます。
3. 手順に従って準備し、Baker Link Envに戻って **Run checks（再診断）** を押します。
4. すべて **Passed（正常）** になったら、**← Back to Home** で戻ります。

**Not checked（未確認）** は、前提となるツールが利用できないなどの理由でまだ診断していない状態です。
まずNeeds attentionの項目を直してください。未確認の項目を別々の故障と考える必要はありません。
PATH（コマンドを探すフォルダー一覧）を変更した場合は、Baker Link Envを完全に終了して起動し直してください。

| Doctorの項目 | 対処方法 |
|---|---|
| VS Code | [VS Codeを入れる・コマンドを有効にする](#doctor-vscode) |
| Dev Containers | [ホスト側に拡張機能を入れる](#doctor-dev-containers) |
| Docker CLI | [Dockerを入れる・コマンドを使えるようにする](#doctor-docker-cli) |
| Docker Engine | [Dockerを起動する・接続先を確認する](#doctor-docker-engine) |
| Host networking support | [対応するDocker環境を選ぶ](#doctor-host-networking) |

通知の **×** で通知だけを閉じられます。閉じても診断結果は残ります。
診断自体が失敗した場合もRun checksで再試行できます。
解決しない場合は **Details（詳細）** を確認し、**Copy report** で結果をコピーして相談してください。
共有前に、個人のフォルダー名やホスト名が含まれていないか確認してください。
オンラインHelpにはインターネット接続が必要です。

## doctor-vscode

### VS Codeを検出できない・実行できない

1. [VS Codeの公式ダウンロードページ](https://code.visualstudio.com/download)から、お使いのOS用のVS Codeをインストールします。
2. VS Codeを一度開き、正常に起動することを確認します。
3. お使いのOSに応じて、`code` コマンドを使えるようにします。
   - **Windows:** インストーラーの **PATHへの追加（Add to PATH）** を有効にします。すでに入っている場合もインストーラーを再実行して確認できます。
   - **macOS:** VS Codeを `/Applications` または `~/Applications` に置きます。別の場所に置く場合は、
     `Cmd+Shift+P` でコマンドパレットを開き、**Shell Command: Install 'code' command in PATH** を実行します。
   - **Debian:** [公式Linux導入手順](https://code.visualstudio.com/docs/setup/linux)に従ってインストールします。
4. ホスト側で新しいターミナルを開き、`code --version` を実行します。
   WindowsではPowerShell、macOS・Debianではターミナルを使ってください。
   バージョン番号が表示されればコマンドを実行できています。
5. Baker Link Envを完全に終了して起動し直し、Doctorで再診断します。

「コマンドが見つからない」場合はPATHを確認します。権限エラーやタイムアウトの場合は、
VS Codeが未インストールとは限らないのでDoctorのDetailsを確認してください。

## doctor-dev-containers

### Dev Containersをホスト側のVS Codeに入れる

1. コンテナに接続していない、通常のVS Codeウィンドウを開きます。
2. **拡張機能** を開きます。Windows・Debianは `Ctrl+Shift+X`、macOSは `Cmd+Shift+X` です。
3. **Dev Containers** を検索し、発行元が **Microsoft** のものを選びます。
   拡張機能IDは `ms-vscode-remote.remote-containers` です。
4. **インストール** を押します。見つからない場合は[公式の拡張機能ページ](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers)を開いてください。
   再読み込みを求められたら実行します。
5. Baker Link Envに戻り、**Doctor → Run checks** を押します。

コンテナ内だけへのインストールでは、この環境診断の条件を満たしません。
拡張一覧の取得に失敗している場合は、先にVS Codeのコマンドを確認してください。
複数のVS Codeやプロファイルを使っている場合、`code` コマンドが使うVS Codeの標準プロファイルにも必要です。

## doctor-docker-cli

### Dockerを入れて、dockerコマンドを使えるようにする

1. 初めてなら、[Rancher Desktopの公式導入手順](https://docs.rancherdesktop.io/getting-started/installation/)から、お使いのOS用をインストールします。
   Docker Desktopも対応しています。Debianでは、ホスト上の通常の[Docker Engine](https://docs.docker.com/engine/install/debian/)も利用できます。
2. Rancher Desktopを起動し、コンテナエンジンを **dockerd (moby)** にします。
   `containerd` のみでは、このアプリで利用するDocker APIを使えません。
3. WindowsでWSL 2の導入や再起動を求められた場合は、公式手順に従って完了してください。
   Docker Desktopを使う場合は **Linux containers** を選びます。
4. ランタイムの案内に従い、Docker CLIをPATHから実行できるようにします。
   WindowsではDocker CLIのフォルダーをユーザーのPATHに登録します。
   macOS・DebianではCLIのリンク作成、または案内されたCLIフォルダーのPATH追加を行います。
5. ホスト側で新しいターミナルを開き、`docker --version` を実行します。バージョンが表示されることを確認します。
6. Baker Link Envを完全に終了して起動し直し、再診断します。

迷う場合は、まず1つのDockerランタイムだけを使ってください。
バージョンが表示されてもDocker本体が起動しているとは限りません。次の項目も確認します。

## doctor-docker-engine

### Dockerは入っているが、接続できない

1. インストールしたRancher DesktopまたはDocker Desktopを開き、起動完了まで待ちます。
   Debianの通常のDocker Engineでは、公式手順に従ってサービスを起動してください。
2. ホスト側のターミナルで `docker info` を実行し、サーバー情報が表示されることを確認します。
3. 接続できない場合は `docker context ls` を実行します。`*` が付いたものが現在の接続先です。
   必要なら `docker context use NAME` を実行します。`NAME` は実際に表示された、自分のランタイムの名前に置き換えてください。
   関係のないリモートサーバーを選ばないでください。
4. Debianでソケットの権限エラーが出る場合は、[公式のインストール後の手順](https://docs.docker.com/engine/install/linux-postinstall/)を確認します。
   Dockerグループへの参加はホストを操作する強い権限を与えるため、注意事項を読んでください。
   指示があればログアウトしてログインし直します。
5. Baker Link Envに戻り、**Doctor → Run checks** を押します。

Detailsに出力形式のエラーやタイムアウトがある場合も、まずターミナルの `docker info` を確認してください。
必要に応じてランタイムを更新・修復して再診断します。Doctor自体はDockerを起動しません。

## doctor-host-networking

### 自動ホスト接続設定に対応する環境を使う

1. Windows・macOSでは、**Rancher DesktopまたはDocker DesktopのLinuxコンテナ**を使います。
   Debianでは、ホスト上で動く通常の、rootlessではないDocker Engineも対応します。
2. Windowsコンテナ、rootlessの通常Docker、リモートの通常Dockerは、自動設定の対象外です。
   別の接続設定が必要になるため、初心者の場合は上記の対応環境を使ってください。
3. `docker context ls` と `docker info` で意図したDocker環境に接続していることを確認し、Doctorを再実行します。

この項目のPassedは「ランタイムが対応している」という意味です。
コンテナからホストのポートに接続できることや、MCUが接続されていることまでは診断しません。
IPアドレスを推測で固定したり、ファイアウォールを無効にしたりしないでください。

## プロジェクトを作成してデバッグする

1. **Create Project** にプロジェクト名を入力し、**Create** で保存先を選びます。
2. VS CodeでDev Containerが起動するまで待ちます。コンテナ側のBaker Link Env拡張も自動でインストールされます。
3. ホスト側のBaker Link Envで **Run** を押し、DAPとELFアップロードのサーバーを起動します。
   標準のBind IPは `0.0.0.0`、ポートは `50001` と `50002` です。
4. 実機とデバッグプローブを接続し、VS Codeで **F5** を押します。

接続先の標準設定は `host.docker.internal:50001` と `http://host.docker.internal:50002` です。
OSやDockerランタイムを変更した場合は、Baker Link Env経由でプロジェクトを開き直し、
VS Codeの **Dev Containers: Rebuild Container** で設定を反映してください。
再作成はコンテナ内のプロセスを停止するため、作業を保存してから行います。

ホストのサービスには認証がないため、信頼できるネットワークと適切なファイアウォール設定で利用してください。
Doctorは環境診断のみで、プロジェクト設定、コンテナ接続、ホストポート、USBデバイスは対象外です。

## インストールと詳細情報

- アプリの配布：[GitHub Releases](https://github.com/Baker-link-Lab/baker-link-env/releases)
- ビルド、拡張機能開発、詳細な接続仕様：[英語README](README.md)

macOSで「壊れているため開けません」と表示される場合の対処は、[英語READMEのmacOS手順](README.md#macos-damaged-app-error)を確認してください。
公式の配布元から取得したアプリであることを確認してから実行してください。
