pub fn setup_ui(ui: &mut egui::Ui, clipboard: &mut arboard::Clipboard) {
    ui.label("Baker link.toolは、Visual Studio CodeのDeveloping inside a Containerを使ったRustの開発環境を構築するツールです。そのためDocker(Rancher Desktop by SUSE)、Visual Studio Code、probe-rsをインストールを前提としています。");
    ui.add_space(2.0);
    ui.heading("Docker");
    let space = 16.0;
    ui.horizontal(|ui| {
        ui.add_space(space);
        ui.hyperlink_to("Rancher Desktop by SUSE", "https://rancherdesktop.io/");
        ui.label("のページに移動して、インストールしてください。");
    });

    ui.add_space(20.0);

    ui.heading("Visual Studio Code");
    ui.horizontal(|ui| {
        ui.add_space(space);
        ui.hyperlink_to("Visual Studio Code", "https://code.visualstudio.com/");
        ui.label("のページに移動して、インストールしてください。");
    });

    ui.add_space(20.0);

    ui.heading("probe-rsのインストール");
    #[cfg(target_os = "windows")]
    {
        ui.horizontal(|ui| {
            ui.add_space(space);
            ui.hyperlink_to("probe-rs", "https://probe.rs/");
            ui.label("が公式のリンクになります。以下、probe-rsのインストール方法について抜粋して記載します。");
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(space);
            ui.label(
                "(1)Power Shell（管理者）を起動して次のコマンドを実行して実行権限を取得します。",
            );
        });
        ui.horizontal(|ui| {
            ui.add_space(space + 15.0);
            ui.label("コマンド実行後にYを入力してEnterを押してください。");
        });
        ui.horizontal(|ui| {
            ui.add_space(space);
            let cmd_text = "Set-ExecutionPolicy RemoteSigned -scope CurrentUser";
            ui.label(egui::RichText::new(cmd_text).size(11.0));
            if ui.button("copy").clicked(){ 
                clipboard.set_text(cmd_text).unwrap();
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            ui.add_space(space);
            ui.label(
                "(2)実行権限を取得した後に次のコマンドを実行してprobe-rsをインストールします。",
            );
        });
        ui.horizontal(|ui|{
            ui.add_space(space);
            let cmd_text = "irm https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.ps1 | iex";
            ui.label(egui::RichText::new(cmd_text).size(11.0));
            if ui.button("copy").clicked(){
                clipboard.set_text(cmd_text).unwrap();
            }
        });
    }
    #[cfg(target_os = "macos")]
    {
        ui.horizontal(|ui| {
            ui.add_space(space);
            ui.hyperlink_to("probe-rs", "https://probe.rs/");
            ui.label("をインストールしてください。");
            ui.label("ターミナルで以下のコマンドを実行してインストールできます。");
        });
        ui.horizontal(|ui|{
            ui.add_space(space);
            let cmd_text = "curl --proto '=https' --tlsv1.2 -LsSf https://github.com/probe-rs/probe-rs/releases/latest/download/probe-rs-tools-installer.sh | sh";
            ui.label(egui::RichText::new(cmd_text).size(11.0));
            if ui.button("copy").clicked(){
                clipboard.set_text(cmd_text).unwrap();
            }
        });
    }
}
