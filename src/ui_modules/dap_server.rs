use crate::cmd;
use crate::logger::DisplayBuffer;
use crate::uiutil;

/// UI for managing probe-rs DAP Server
pub struct DapServerPanel;

impl DapServerPanel {
    pub fn show(
        ui: &mut egui::Ui,
        probe_rs_dap_server: &mut cmd::ProbeRsDapServer,
        display_buffer: &mut DisplayBuffer,
    ) {
        ui.label(uiutil::make_section_title("probe-rs DAP Server"));
        ui.label(uiutil::make_section_subtitle(
            "Launch a local DAP server for debugging.",
        ));
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label("Port");
            ui.add(
                egui::TextEdit::singleline(&mut probe_rs_dap_server.port)
                    .desired_width(80.0)
                    .hint_text("50001"),
            );
            ui.add_space(8.0);
            let can_run = probe_rs_dap_server.status == cmd::DapServerStatus::Stopped;
            let can_stop = !can_run;

            if ui
                .add_enabled(can_run, uiutil::make_primary_button("Run"))
                .clicked()
            {
                match probe_rs_dap_server.start(display_buffer.tx.clone()) {
                    Ok(()) => display_buffer.log_info(format!(
                        "probe-rs DAP Server started on port {}",
                        probe_rs_dap_server.port
                    )),
                    Err(error) => display_buffer.log_error(error),
                }
            };
            if ui
                .add_enabled(can_stop, uiutil::make_danger_button("Stop"))
                .clicked()
            {
                if probe_rs_dap_server.stop() {
                    display_buffer.log_info("probe-rs DAP Server stopped".to_string());
                };
            }
        });
    }
}
