use crate::cmd;
use crate::infoui::setup_ui;
use crate::logger::DisplayBuffer;
use crate::parameter;
use crate::uiutil;
use crate::uiutil::make_orange_button;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct EvnApp {
    new_project: cmd::NewProject,
    probe_rs_dap_server: cmd::ProbeRsDapServer,
    #[serde(skip)]
    display_buffer: DisplayBuffer,
    info: bool,
    #[serde(skip)]
    clipboard: arboard::Clipboard,
}

impl Default for EvnApp {
    fn default() -> Self {
        Self {
            new_project: Default::default(),
            probe_rs_dap_server: Default::default(),
            display_buffer: DisplayBuffer::new(),
            info: true,
            clipboard: arboard::Clipboard::new().unwrap(),
        }
    }
}

impl EvnApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }
}

impl EvnApp {
    fn project_create_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Create Project");
        ui.horizontal(|ui| {
            ui.label("Project name:");
            ui.add(egui::TextEdit::singleline(&mut self.new_project.name).desired_width(100.0));
            if ui.add(make_orange_button("create")).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    self.new_project.path = path.to_string_lossy().to_string();
                    if cmd::is_folder_exists(&format!(
                        "{}/{}",
                        self.new_project.path.clone(),
                        self.new_project.name.clone()
                    )) {
                        match cmd::generate_project(&self.new_project.name, &self.new_project.path)
                        {
                            Ok(_) => {
                                self.display_buffer.log_info(format!(
                                    "Project {}/{} generated",
                                    self.new_project.path, self.new_project.name
                                ));
                            }
                            Err(e) => {
                                self.display_buffer.log_error(format!(
                                    "Project {}/{} generate failed: {}",
                                    self.new_project.path, self.new_project.name, e
                                ));
                            }
                        }
                    } else {
                        self.display_buffer.log_info(format!(
                            "Project {}/{} already created",
                            self.new_project.path, self.new_project.name
                        ));
                    }
                    if self.new_project.vscode_open_enabled {
                        match cmd::open_vscode(&format!(
                            "{}/{}",
                            &self.new_project.path, &self.new_project.name
                        )) {
                            Ok(_) => {
                                self.display_buffer.log_info(format!(
                                    "Visual Studio Code opened: {}/{}",
                                    self.new_project.path, self.new_project.name
                                ));
                            }
                            Err(e) => {
                                self.display_buffer.log_error(format!(
                                    "Visual Studio Code open failed: {}/{}: {}",
                                    self.new_project.path, self.new_project.name, e
                                ));
                            }
                        };
                    }
                }
            }
        });
        ui.checkbox(
            &mut self.new_project.vscode_open_enabled,
            "Visual Studio Code open",
        );
        ui.hyperlink_to(
            "Template Code",
            "https://github.com/T-ikko/bakerlink_tutorial_template.git",
        );
    }

    fn probe_rs_dap_server_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("probe-rs DAP Server");

        let mut run_color = egui::Color32::GRAY;
        let mut stop_color = egui::Color32::GRAY;
        if self.probe_rs_dap_server.status == cmd::DapServerStatus::Stopped {
            run_color = egui::Color32::GREEN;
        } else {
            stop_color = egui::Color32::RED;
        }

        ui.horizontal(|ui| {
            ui.label("Port:");
            ui.add(
                egui::TextEdit::singleline(&mut self.probe_rs_dap_server.port).desired_width(50.0),
            );
            ui.add_space(1.0);
            if ui.add(egui::Button::new("Run").fill(run_color)).clicked() {
                let probe_rs_versoin = cmd::get_probe_rs_versions();
                if probe_rs_versoin.is_none() {
                    self.display_buffer
                        .log_error("probe-rs not found".to_string());
                } else {
                    self.probe_rs_dap_server
                        .start(self.display_buffer.tx.clone());
                }
            };
            if ui.add(egui::Button::new("Stop").fill(stop_color)).clicked() {
                if self.probe_rs_dap_server.stop() {
                    self.display_buffer
                        .log_info("probe-rs dap-server stopped".to_string());
                };
            }
        });
    }
}

impl eframe::App for EvnApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("help").clicked() {
                    self.info = !self.info;
                };
                ui.menu_button("history", |ui| {
                    for (i, path) in self.new_project.history.clone().iter().enumerate() {
                        if ui.button(path).clicked() {
                            if cmd::is_folder_exists(path) {
                                cmd::open_vscode(path);
                            } else {
                                self.display_buffer.log_error(format!(
                                    "Project not found: {}",
                                    path
                                ));
                                self.new_project.history.remove(i);
                            }
                        }
                    }
                });
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new(parameter::APP_NAME)
                        .color(egui::Color32::from_hex("#DD7032").unwrap())
                        .size(32.0),
                );
            });

            let frame_pj_create = uiutil::get_frame(ui);
            frame_pj_create.show(ui, |ui| {
                self.project_create_ui(ui);
            });

            let probers_dapserver_frame = uiutil::get_frame(ui);
            probers_dapserver_frame.show(ui, |ui| {
                self.probe_rs_dap_server_ui(ui);
            });

            egui::TopBottomPanel::bottom("bottom_panel").show_inside(ui, |ui| {
                ui.heading("Log");
                self.display_buffer.channel_recv();

                let text_style = egui::TextStyle::Body;
                let row_height = ui.text_style_height(&text_style);
                let num_rows = self.display_buffer.buffer.len();
                egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                    ui,
                    row_height,
                    num_rows,
                    |ui, row_range| {
                        egui::Frame::none().inner_margin(10.0).show(ui, |ui| {
                            for i in row_range {
                                ui.label(self.display_buffer.buffer[i].clone());
                                ui.add_space(0.12);
                            }
                        });
                    },
                );
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    powered_by_egui_and_eframe(ui);
                });
            });
        });

        if self.info {
            egui::Window::new("Info")
                .open(&mut self.info)
                .default_width(400.0)
                .show(ctx, |ui| {
                    setup_ui(ui, &mut self.clipboard);
                });
        }
    }
}

fn powered_by_egui_and_eframe(ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.label("Powered by ");
        ui.hyperlink_to("egui", "https://github.com/emilk/egui");
        ui.label(" and ");
        ui.hyperlink_to(
            "eframe",
            "https://github.com/emilk/egui/tree/master/crates/eframe",
        );
        ui.label(".");
    });
}
