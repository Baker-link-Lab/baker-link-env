use crate::cmd;
use crate::logger::DisplayBuffer;
use crate::parameter;
use crate::util;
use crate::util::make_orange_button;

#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)] // if we add new fields, give them default values when deserializing old state
pub struct EvnApp {
    new_project: cmd::NewProject,
    probe_rs_dap_server: cmd::ProbeRsDapServer,
    #[serde(skip)]
    display_buffer: DisplayBuffer,
    #[serde(skip)]
    value: f32,
}

impl Default for EvnApp {
    fn default() -> Self {
        Self {
            // Example stuff:
            new_project: Default::default(),
            probe_rs_dap_server: Default::default(),
            value: 2.7,
            display_buffer: DisplayBuffer::new(),
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
                    println!("path: {}/{}", self.new_project.path, self.new_project.name);
                    if self.new_project.history_push( format!("{}/{}", self.new_project.path.clone(), self.new_project.name.clone())) {
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
                        if std::path::Path::new(&format!(
                            "{}/{}",
                            &self.new_project.path, &self.new_project.name
                        ))
                        .exists()
                        {
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
                                    self.display_buffer.log_info(
                                        (format!(
                                            "Visual Studio Code open failed: {}/{}: {}",
                                            self.new_project.path, self.new_project.name, e
                                        )),
                                    );
                                }
                            };
                        } else {
                            self.display_buffer.log_error(format!(
                                "Visual Studio Code open failed: {}/{} not exists",
                                self.new_project.path, self.new_project.name
                            ));
                        }
                    }
                }
            }
        });
        ui.checkbox(
            &mut self.new_project.vscode_open_enabled,
            "Visual Studio Code open",
        );
    }

    fn probe_rs_dap_server_ui(&mut self, ui: &mut egui::Ui) {
        ui.heading("Probe-rs DAP Server");
        ui.horizontal(|ui| {
            ui.label("Port:");
            ui.add(
                egui::TextEdit::singleline(&mut self.probe_rs_dap_server.port).desired_width(50.0),
            );
            ui.add_space(1.0);
            ui.button("Run");
            ui.button("Stop");
        });
    }
}

impl eframe::App for EvnApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("top_panel").show(ctx, |ui| {
            ui.button("help");
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading(
                    egui::RichText::new(parameter::APP_NAME)
                        .color(egui::Color32::from_hex("#DD7032").unwrap())
                        .size(32.0),
                );
            });

            let frame_pj_create = util::get_frame(ui);
            frame_pj_create.show(ui, |ui| {
                self.project_create_ui(ui);
            });

            let probers_dapserver_frame = util::get_frame(ui);
            probers_dapserver_frame.show(ui, |ui| {
                self.probe_rs_dap_server_ui(ui);
            });

            // ui.add(egui::github_link_file!(
            //     "https://github.com/emilk/eframe_template/blob/main/",
            //     "Source code."
            // ));

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
                    egui::warn_if_debug_build(ui);
                });
            });
        });
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
