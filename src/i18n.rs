use std::borrow::Cow;

#[derive(Clone, Copy, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    English,
    Japanese,
}

impl Language {
    pub fn help_url(self, section: Option<&str>) -> String {
        let file = match self {
            Self::English => "README.md",
            Self::Japanese => "README.ja.md",
        };
        let mut url = format!("https://github.com/Baker-link-Lab/baker-link-env/blob/main/{file}");
        if let Some(section) = section {
            url.push('#');
            url.push_str(section);
        }
        url
    }

    pub fn text<'a>(self, english: &'a str) -> Cow<'a, str> {
        if self == Self::English {
            return Cow::Borrowed(english);
        }
        for &(source, translation) in TEXT {
            if source == english {
                return Cow::Borrowed(translation);
            }
        }
        // Application messages are translated; command output, paths and device names stay intact.
        for &(source, translation) in TEMPLATES {
            let parts: Vec<_> = source.split("{}").collect();
            if let Some(values) = template_values(english, &parts) {
                let mut translated = String::new();
                let mut translations = translation.split("{}");
                translated.push_str(translations.next().unwrap_or(""));
                for (value, tail) in values.into_iter().zip(translations) {
                    translated.push_str(&self.text(value));
                    translated.push_str(tail);
                }
                return Cow::Owned(translated);
            }
        }
        if english.contains('\n') {
            return Cow::Owned(
                english
                    .split('\n')
                    .map(|line| self.text(line))
                    .collect::<Vec<_>>()
                    .join("\n"),
            );
        }
        Cow::Borrowed(english)
    }
}

fn template_values<'a>(text: &'a str, parts: &[&str]) -> Option<Vec<&'a str>> {
    let mut remaining = text.strip_prefix(parts[0])?;
    let mut values = Vec::new();
    for (index, part) in parts.iter().enumerate().skip(1) {
        if index == parts.len() - 1 {
            values.push(remaining.strip_suffix(part)?);
        } else {
            let (value, rest) = remaining.split_once(part)?;
            values.push(value);
            remaining = rest;
        }
    }
    Some(values)
}

const TEXT: &[(&str, &str)] = &[
    ("Language", "表示言語"),
    ("Help", "ヘルプ"),
    ("Home", "ホーム"),
    ("Doctor", "環境診断"),
    ("History", "履歴"),
    ("No history yet", "履歴はありません"),
    ("Reset all settings", "すべての設定をリセット"),
    ("Return to Home", "ホームへ戻る"),
    ("\u{2190} Back to Home", "\u{2190} ホームへ戻る"),
    ("Check your development tools and Docker runtime. No projects or settings are changed.", "開発ツールとDocker環境を確認します。プロジェクトや設定は変更しません。"),
    ("Checking...", "診断中..."),
    ("Run checks", "再診断"),
    ("Checking VS Code, Dev Containers, and Docker. Each command has a 15-second timeout.", "VS Code、Dev Containers、Dockerを確認しています。各コマンドのタイムアウトは15秒です。"),
    ("Previous result (checks are running)", "前回の結果（診断中）"),
    ("This checks host environment readiness only, not project configuration, container connections, or USB devices.", "ホストの開発環境のみを診断します。プロジェクト設定、コンテナ接続、USBデバイスは対象外です。"),
    ("Start with a Needs attention item and click Read setup steps. Help opens at the matching section in your browser. Follow the steps, return here, and click Run checks.", "「対応が必要」の項目で「解決手順を読む」を押してください。日本語ヘルプの該当箇所がブラウザーで開きます。対処後は、この画面で「再診断」を押してください。"),
    ("Start here: environment setup", "はじめての環境セットアップ"),
    ("Read setup steps", "解決手順を読む"),
    ("Copy report", "診断結果をコピー"),
    ("Details", "詳細"),
    ("Run checks to see what is ready and what needs attention. Docker does not need to be running to start the diagnosis.", "診断を実行すると準備状況と対処が必要な項目を確認できます。Dockerが停止中でも診断を開始できます。"),
    ("Some environment checks need your attention. Open the setup steps for the failed item in Doctor.", "開発環境に対処が必要な項目があります。環境診断で該当項目の解決手順を開いてください。"),
    ("Open Help", "ヘルプを開く"),
    ("Create Project", "プロジェクト作成"),
    ("Generate a template-based project and open it in VS Code.", "テンプレートからプロジェクトを作成し、VS Codeで開きます。"),
    ("Project name", "プロジェクト名"),
    ("Create", "作成"),
    ("Specify template version", "テンプレートのバージョンを指定"),
    ("Version", "バージョン"),
    ("Branch", "ブランチ"),
    ("Tag", "タグ"),
    ("e.g. v1.0.0 (blank = latest)", "例: v1.0.0（空欄 = 最新）"),
    ("e.g. main (blank = HEAD)", "例: main（空欄 = HEAD）"),
    ("Open VS Code after creation", "作成後にVS Codeを開く"),
    ("View template repository", "テンプレートのリポジトリを見る"),
    ("probe-rs DAP Server", "probe-rs DAPサーバー"),
    ("Launch a local DAP server for debugging.", "デバッグ用のDAPサーバーをホスト上で起動します。"),
    ("Running", "実行中"),
    ("Stopped", "停止中"),
    ("Connected Device", "接続デバイス"),
    ("Detecting...", "検出中..."),
    ("Detect", "検出"),
    ("Probe", "プローブ"),
    ("Chip", "チップ"),
    ("Core", "コア"),
    ("Bind IP", "待受IP"),
    ("DAP", "DAP"),
    ("Upload", "アップロード"),
    ("Run", "実行"),
    ("Stop", "停止"),
    ("Stopping...", "停止処理中..."),
    ("Dev Container connects through host.docker.internal. Changes apply on Run.", "Dev Containerはhost.docker.internal経由で接続します。変更は「実行」時に適用されます。"),
    ("Log", "ログ"),
    ("Build and runtime output.", "ビルド・実行時の出力。"),
    ("Copy", "コピー"),
    ("Dismiss notification", "通知を閉じる"),
    ("Dismiss Docker startup prompt", "Dockerの起動確認を閉じる"),
    ("Cancel reset", "リセットを取り消す"),
    ("Rancher Desktop", "Rancher Desktop"),
    ("Docker is not running.", "Dockerが起動していません。"),
    ("Start Rancher Desktop now?", "Rancher Desktopを起動しますか？"),
    ("Start", "起動"),
    ("Not now", "今は起動しない"),
    ("Reset Settings", "設定をリセット"),
    ("All settings including project history will be cleared.", "表示言語とプロジェクト履歴を含むすべての設定が削除されます。"),
    ("The application will restart from the splash screen.", "初期画面に戻ります。"),
    ("Reset", "リセット"),
    ("Cancel", "キャンセル"),
    ("Docker: On", "Docker: 起動中"),
    ("Docker: Off", "Docker: 停止中"),
    ("Docker: ?", "Docker: 未確認"),
    ("No probe found", "プローブが見つかりません"),
    ("Detection failed", "検出に失敗しました"),
    ("Passed", "正常"),
    ("Needs attention", "対応が必要"),
    ("Not checked", "未確認"),
    ("VS Code", "VS Code"),
    ("Dev Containers", "Dev Containers"),
    ("Docker CLI", "Docker CLI"),
    ("Docker Engine", "Dockerエンジン"),
    ("Host networking support", "ホスト接続設定への対応"),
    ("Ready to create projects.", "プロジェクトを作成できます。"),
    ("Some checks need your attention. Follow the steps below.", "対処が必要な項目があります。以下の手順を確認してください。"),
    ("VS Code CLI is available.", "VS Codeのコマンドを実行できます。"),
    ("Dev Containers is installed in the host VS Code.", "ホスト側のVS CodeにDev Containersがインストールされています。"),
    ("Install the Dev Containers extension in the host VS Code.", "ホスト側のVS CodeにDev Containers拡張機能をインストールしてください。"),
    ("Could not list host VS Code extensions. Check the CLI error below.", "ホスト側のVS Codeの拡張一覧を取得できません。詳細のコマンドエラーを確認してください。"),
    ("Could not run VS Code. Install it or make its code CLI available.", "VS Codeを実行できません。インストールまたはcodeコマンドの設定を確認してください。"),
    ("Check VS Code first, then run Doctor again.", "先にVS Codeを確認し、その後に環境診断を再実行してください。"),
    ("Could not run Docker. Install a Docker runtime and check PATH.", "Dockerを実行できません。Docker環境のインストールとPATHを確認してください。"),
    ("Docker CLI is required to check the engine.", "エンジンの確認にはDocker CLIが必要です。"),
    ("Docker Engine information is required.", "Dockerエンジンの情報が必要です。"),
    ("Docker CLI is available.", "Dockerのコマンドを実行できます。"),
    ("Cannot reach Docker Engine. Start your runtime and check the active Docker context.", "Dockerエンジンに接続できません。Docker環境を起動し、選択中のDockerコンテキストを確認してください。"),
    ("Start Docker Engine, then run Doctor again.", "Dockerエンジンを起動し、環境診断を再実行してください。"),
    ("Docker returned invalid runtime information.", "Dockerから取得した環境情報の形式が不正です。"),
    ("Valid Docker Engine information is required.", "正しいDockerエンジン情報が必要です。"),
    ("Docker Engine is reachable.", "Dockerエンジンに接続できます。"),
    ("Local Docker Engine supports the Linux host-gateway setup.", "ローカルのDockerエンジンはLinux用のhost-gateway設定に対応しています。"),
    ("Desktop runtime provides native host DNS.", "デスクトップ版Docker環境がホストの名前解決を提供します。"),
    ("Runtime compatibility only; no container or host port connection was tested.", "実行環境の対応状況のみを確認しています。コンテナやホストのポートへの接続は未検証です。"),
    ("This runtime needs a different host connection setup.", "この実行環境では別のホスト接続設定が必要です。"),
    ("Baker Link Dev Containers require a Linux Docker engine", "Baker LinkのDev ContainerにはLinuxのDockerエンジンが必要です"),
    ("Rootless Docker requires an explicit host connection setup; automatic host-gateway configuration is not supported", "rootless Dockerには個別のホスト接続設定が必要です。host-gatewayの自動設定には対応していません"),
    ("Automatic host-gateway setup requires a local Docker socket; remote Docker endpoints need explicit host connection settings", "host-gatewayの自動設定にはローカルのDockerソケットが必要です。リモートDockerには個別の接続設定が必要です"),
    ("Command returned an empty result.", "コマンドの結果が空でした。"),
    ("Rancher Desktop started", "Rancher Desktopを起動しました"),
    ("Stopping probe-rs DAP and ELF upload servers...", "probe-rs DAP・ELFアップロードサーバーを停止しています..."),
    ("probe-rs DAP and ELF upload servers stopped", "probe-rs DAP・ELFアップロードサーバーを停止しました"),
    ("Starting Rancher Desktop; waiting for Docker...", "Rancher Desktopを起動し、Dockerの準備を待っています..."),
    ("Dev Container host networking configured. Rebuild an existing container to apply changes.", "Dev Containerのホスト接続設定を更新しました。既存のコンテナは再作成すると反映されます。"),
    ("Rancher Desktop start: channel closed", "Rancher Desktopの起動処理との通信が終了しました"),
    ("ERR", "エラー"),
    ("INF", "情報"),
    ("LOG", "ログ"),
    ("ERROR", "エラー"),
    ("Baker Link Doctor", "Baker Link 環境診断"),
    ("Choose project folder", "プロジェクトの保存先を選択"),
    ("Invalid port number", "DAPポート番号が不正です"),
    ("Invalid ELF upload port number", "ELFアップロードのポート番号が不正です"),
    ("Invalid IP address", "IPアドレスが不正です"),
    ("DAP port and ELF upload port must be different", "DAPとELFアップロードのポート番号は別々にしてください"),
    ("Install the Dev Containers extension (ms-vscode-remote.remote-containers) in VS Code", "VS CodeにDev Containers拡張機能（ms-vscode-remote.remote-containers）をインストールしてください"),
    ("Docker did not become ready within 60 seconds. Start Rancher Desktop and try again", "60秒以内にDockerの準備が完了しませんでした。Rancher Desktopを起動して再試行してください"),
    ("Failed to execute VS Code CLI; ensure Visual Studio Code is installed and its code command is available", "VS Code CLIを実行できません。VS Codeのインストールとcodeコマンドの設定を確認してください"),
    ("Failed to inspect Docker host networking", "Dockerのホスト接続情報を取得できません"),
    ("Failed to inspect Docker context", "Dockerの接続先を取得できません"),
    ("Invalid Docker runtime information", "Dockerの環境情報が不正です"),
    ("Invalid Docker context endpoint", "Dockerの接続先情報が不正です"),
    ("Project path is not valid UTF-8", "プロジェクトのパスをUTF-8として読み取れません"),
    ("No debug probe found", "デバッグプローブが見つかりません"),
    ("DAP server shutdown requested", "DAPサーバーの停止要求を受信しました"),
    ("Only PUT is supported", "PUTメソッドのみ対応しています"),
    ("Missing X-Content-SHA256 header", "X-Content-SHA256ヘッダーがありません"),
    ("Missing Content-Length header", "Content-Lengthヘッダーがありません"),
    ("Invalid upload URL", "アップロードURLが不正です"),
    ("Missing project identifier", "プロジェクト識別子がありません"),
    ("Missing profile identifier", "プロファイル識別子がありません"),
    ("Could not determine the operating system cache directory", "OSのキャッシュフォルダーを取得できません"),
    ("Artifact path has no parent", "成果物のパスに親フォルダーがありません"),
    ("Artifact body is larger than Content-Length", "成果物のデータがContent-Lengthより大きいです"),
    ("Artifact SHA-256 mismatch", "成果物のSHA-256が一致しません"),
    ("Invalid SHA-256 header", "SHA-256ヘッダーが不正です"),
    ("Project path has no folder name", "プロジェクトのパスにフォルダー名がありません"),
    ("Set an explicit workspaceFolder in devcontainer.json to open directly in a container", "コンテナで直接開くにはdevcontainer.jsonでworkspaceFolderを指定してください"),
    ("workspaceFolder must be an absolute container path; only ${localWorkspaceFolderBasename} substitution is supported", "workspaceFolderにはコンテナ内の絶対パスが必要です。変数は${localWorkspaceFolderBasename}のみ対応しています"),
];

const TEMPLATES: &[(&str, &str)] = &[
    ("Last checked: {}", "最終診断: {}"),
    ("Host: {}", "ホスト: {}"),
    ("Checked: {}", "診断日時: {}"),
    ("Runtime: {}", "実行環境: {}"),
    ("Doctor: {}", "環境診断: {}"),
    ("Doctor / {}: {}", "環境診断 / {}: {}"),
    (
        "Failed to open Help: {}. Open {} in your browser.",
        "ヘルプを開けません: {}。ブラウザーで {} を開いてください。",
    ),
    (
        "Failed to save language settings: {}",
        "表示言語を保存できません: {}",
    ),
    (
        "Failed to load language settings: {}",
        "表示言語を読み込めません: {}",
    ),
    (
        "Environment diagnosis failed: {}",
        "環境診断に失敗しました: {}",
    ),
    (
        "Failed to save server settings: {}",
        "サーバー設定を保存できません: {}",
    ),
    (
        "Failed to load server settings: {}",
        "サーバー設定を読み込めません: {}",
    ),
    (
        "Failed to load project history: {}",
        "プロジェクト履歴を読み込めません: {}",
    ),
    (
        "Failed to save project history: {}",
        "プロジェクト履歴を保存できません: {}",
    ),
    (
        "Failed to load splash state: {}",
        "初期画面の設定を読み込めません: {}",
    ),
    (
        "Failed to save splash state: {}",
        "初期画面の設定を保存できません: {}",
    ),
    (
        "Failed to reset settings: {}",
        "設定をリセットできません: {}",
    ),
    (
        "Failed to copy Doctor report: {}",
        "診断結果をコピーできません: {}",
    ),
    ("Failed to copy logs: {}", "ログをコピーできません: {}"),
    (
        "probe-rs DAP Server started on {}",
        "probe-rs DAPサーバーを起動しました: {}",
    ),
    (
        "Rancher Desktop start failed: {}",
        "Rancher Desktopの起動に失敗しました: {}",
    ),
    ("Project {} generated", "プロジェクト {} を作成しました"),
    (
        "Project {} already exists",
        "プロジェクト {} はすでに存在します",
    ),
    (
        "Visual Studio Code open requested: {}",
        "VS Codeで開く要求を送信しました: {}",
    ),
    (
        "Visual Studio Code failed to open: {}",
        "VS Codeで開けません: {}",
    ),
    (
        "Project generation failed: {}",
        "プロジェクト作成に失敗しました: {}",
    ),
    (
        "Project creation worker failed: {}",
        "プロジェクト作成処理に失敗しました: {}",
    ),
    (
        "VS Code worker failed: {}",
        "VS Codeの処理に失敗しました: {}",
    ),
    ("Project not found: {}", "プロジェクトが見つかりません: {}"),
    ("Detected: {}", "検出しました: {}"),
    ("Device detection: {}", "デバイス検出: {}"),
    (
        "Failed to execute command: {}",
        "コマンドを実行できません: {}",
    ),
    (
        "Command timed out after {} seconds.",
        "{}秒でコマンドがタイムアウトしました。",
    ),
    (
        "Command failed ({}).\n{}",
        "コマンドが失敗しました（{}）。\n{}",
    ),
    (
        "Command returned no result: {}",
        "コマンドの結果がありません: {}",
    ),
    (
        "Failed to inspect command: {}",
        "コマンドの状態を確認できません: {}",
    ),
    (
        "Failed to read command stdout: {}",
        "標準出力を読み込めません: {}",
    ),
    (
        "Failed to read command stderr: {}",
        "標準エラー出力を読み込めません: {}",
    ),
    (
        "Unsupported Docker runtime on {}: {}",
        "{}で非対応のDocker環境です: {}",
    ),
    (
        "Project folder does not exist: {}",
        "プロジェクトフォルダーが存在しません: {}",
    ),
    (
        "Docker host networking inspection failed: {}",
        "Dockerのホスト接続情報を取得できません: {}",
    ),
    (
        "Docker context inspection failed: {}",
        "Dockerの接続先を取得できません: {}",
    ),
    ("Failed to open probe: {}", "プローブに接続できません: {}"),
    (
        "Failed to detect target: {}",
        "ターゲットを検出できません: {}",
    ),
    ("DAP server is currently {}", "DAPサーバーの現在の状態: {}"),
    (
        "DAP port {} is unavailable: {}",
        "DAPポート {} は使用できません: {}",
    ),
    (
        "ELF upload port {} is unavailable: {}",
        "ELFアップロードポート {} は使用できません: {}",
    ),
    (
        "ELF upload server listening on {}",
        "ELFアップロードサーバーの待受先: {}",
    ),
    (
        "[ERROR] ELF upload server stopped: {}",
        "[エラー] ELFアップロードサーバーが停止しました: {}",
    ),
    (
        "[ERROR] DAP server stopped: {}",
        "[エラー] DAPサーバーが停止しました: {}",
    ),
    (
        "Failed to start runtime: {}",
        "ランタイムを起動できません: {}",
    ),
    ("docker info failed: {}", "Docker情報を取得できません: {}"),
    ("Error: {}", "エラー: {}"),
    ("ELF upload failed: {}", "ELFアップロードに失敗しました: {}"),
    ("ELF uploaded: {}", "ELFをアップロードしました: {}"),
    ("[ERROR] ELF upload rejected: {}", "[エラー] ELFアップロードを拒否しました: {}"),
    ("Failed to bind ELF upload server on {}: {}", "ELFアップロードサーバーを {} で待受できません: {}"),
    ("VS Code CLI failed ({}): {}", "VS Code CLIが失敗しました（{}）: {}"),
    ("Failed to execute VS Code CLI; ensure Visual Studio Code is installed and its code command is available: {}", "VS Code CLIを実行できません。VS Codeのインストールとcodeコマンドの設定を確認してください: {}"),
    ("Invalid {} identifier", "{}の識別子が不正です"),
    ("Failed to read {}", "{}を読み込めません"),
    ("Failed to write {}", "{}に書き込めません"),
    ("Failed to create {}", "{}を作成できません"),
    ("Failed to parse {}", "{}の形式を読み取れません"),
    ("Failed to remove {}", "{}を削除できません"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn switching_translates_existing_messages_and_preserves_technical_values() {
        assert_eq!(
            Language::Japanese.text("Project C:\\work\\myproject generated"),
            "プロジェクト C:\\work\\myproject を作成しました"
        );
        assert_eq!(
            Language::Japanese.text("Doctor: Ready to create projects."),
            "環境診断: プロジェクトを作成できます。"
        );
        assert_eq!(Language::English.text("Run"), "Run");
        assert_eq!(
            Language::Japanese.text("host.docker.internal:50002"),
            "host.docker.internal:50002"
        );
        assert_eq!(
            Language::Japanese
                .text("Failed to open Help: timeout. Open https://example.com in your browser."),
            "ヘルプを開けません: timeout。ブラウザーで https://example.com を開いてください。"
        );
    }

    #[test]
    fn catalog_has_unique_keys_and_matching_template_placeholders() {
        let mut keys = std::collections::HashSet::new();
        for &(english, japanese) in TEXT.iter().chain(TEMPLATES) {
            assert!(keys.insert(english), "duplicate: {english}");
            assert!(!japanese.is_empty());
            assert_eq!(
                english.matches("{}").count(),
                japanese.matches("{}").count()
            );
        }
    }

    #[test]
    fn ui_uses_catalog_for_every_display_string() {
        let app = include_str!("app.rs");
        let ui = app.split_once("    rsx! {").unwrap().1;
        for line in ui.lines() {
            let line = line.trim();
            if line.starts_with('"') && line.ends_with('"') && !line.contains('{') {
                let english = &line[1..line.len() - 1];
                assert!(
                    ["B", r"\u{00d7}"].contains(&english),
                    "unlocalized UI: {english}"
                );
            }
        }
        for source in [app, include_str!("doctor.rs")] {
            for part in source.split(".text(\"").skip(1) {
                let key = part.split('"').next().unwrap();
                assert!(
                    TEXT.iter().any(|entry| entry.0 == key) || key == r"\u{2190} Back to Home",
                    "missing translation: {key}"
                );
            }
        }
    }
}
