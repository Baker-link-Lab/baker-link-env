use crate::cmd;
use crate::logger::DisplayBuffer;
use crate::parameter;
use crate::ui_modules::{DapServerPanel, LoggerPanel, ProjectCreatePanel};
use crate::uiutil;
use std::time::{Duration, Instant};

const HELP_URL: &str = "https://github.com/Baker-link-Lab/baker-link-env/blob/main/README.md";

static INIT: std::sync::Once = std::sync::Once::new();

/// Main application state
#[derive(serde::Deserialize, serde::Serialize)]
#[serde(default)]
pub struct EvnApp {
    new_project: cmd::NewProject,
    probe_rs_dap_server: cmd::ProbeRsDapServer,
    #[serde(skip)]
    display_buffer: DisplayBuffer,
    info: bool,
    #[serde(skip)]
    last_error: Option<String>,
    #[serde(skip)]
    docker_status: DockerStatus,
    #[serde(skip)]
    last_docker_check: Instant,
    #[serde(skip)]
    docker_prompt_dismissed: bool,
}

impl Default for EvnApp {
    fn default() -> Self {
        Self {
            new_project: Default::default(),
            probe_rs_dap_server: Default::default(),
            display_buffer: DisplayBuffer::new(),
            info: true,
            last_error: None,
            docker_status: DockerStatus::Unknown,
            last_docker_check: Instant::now(),
            docker_prompt_dismissed: false,
        }
    }
}

impl EvnApp {
    /// Create a new app instance from creation context
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        uiutil::set_fonts(cc);
        uiutil::apply_theme(cc);

        if let Some(storage) = cc.storage {
            return eframe::get_value(storage, eframe::APP_KEY).unwrap_or_default();
        }
        Default::default()
    }

    /// Initialize the application (run once)
    fn initialize(&mut self) {
        INIT.call_once(|| {
            if cmd::are_apps_runnning("baker-link-env") {
                self.last_error = Some("baker-link-env is already running".to_string());
                self.display_buffer
                    .log_error("baker-link-env is already running".to_string());
            }
        });
    }

    fn update_docker_status(&mut self) {
        if self.last_docker_check.elapsed() < Duration::from_secs(3) {
            return;
        }
        self.last_docker_check = Instant::now();
        match cmd::is_docker_running() {
            Ok(true) => {
                self.docker_status = DockerStatus::Running;
                self.docker_prompt_dismissed = false;
            }
            Ok(false) => self.docker_status = DockerStatus::Stopped,
            Err(_) => self.docker_status = DockerStatus::Unknown,
        }
    }

    fn show_docker_prompt(&mut self, ctx: &egui::Context) {
        if self.docker_status != DockerStatus::Stopped || self.docker_prompt_dismissed {
            return;
        }

        egui::Window::new("Rancher Desktop")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Docker is not running.");
                ui.label("Start Rancher Desktop now?");
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.add(uiutil::make_primary_button("Start")).clicked() {
                        match cmd::start_rd() {
                            Ok(_) => {
                                self.display_buffer
                                    .log_info("Rancher Desktop started".to_string());
                                self.docker_prompt_dismissed = true;
                            }
                            Err(e) => {
                                self.last_error =
                                    Some(format!("Rancher Desktop start failed: {}", e));
                                self.display_buffer
                                    .log_error(format!("Rancher Desktop start failed: {}", e));
                            }
                        }
                        self.last_docker_check = Instant::now() - Duration::from_secs(4);
                    }
                    if ui.add(uiutil::make_chip_button("Not now")).clicked() {
                        self.docker_prompt_dismissed = true;
                    }
                });
            });
    }

    /// Show the top panel with help and history menu
    fn show_top_panel(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("top_panel")
            .frame(uiutil::header_frame())
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(uiutil::make_primary_heading(parameter::APP_NAME));
                    });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let (status_label, status_color) = match self.docker_status {
                            DockerStatus::Running => ("Docker: On", uiutil::colors::STATUS_ON),
                            DockerStatus::Stopped => ("Docker: Off", uiutil::colors::STATUS_OFF),
                            DockerStatus::Unknown => ("Docker: ?", uiutil::colors::STATUS_UNKNOWN),
                        };
                        ui.horizontal(|ui| {
                            ui.label(status_label);
                            uiutil::status_dot(ui, status_color);
                        });
                        ui.add_space(8.0);

                        if ui.add(uiutil::make_chip_button("Help")).clicked() {
                            let _ = open::that(HELP_URL);
                        }

                        ui.menu_button("History", |ui| {
                            for (i, pj) in self.new_project.history.clone().iter().enumerate() {
                                if ui.button(&pj.get_path()).clicked() {
                                    if pj.is_folder_exists() {
                                        let _ = cmd::start_rd();
                                        match cmd::open_vscode(&pj.get_path()) {
                                            Ok(_) => {
                                                self.display_buffer.log_info(format!(
                                                    "Visual Studio Code opened: {}",
                                                    pj.get_path()
                                                ));
                                            }
                                            Err(e) => {
                                                self.last_error =
                                                    Some(format!("VSCode open failed: {}", e));
                                                self.display_buffer.log_error(format!(
                                                    "Visual Studio Code failed to open: {}: {}",
                                                    pj.get_path(),
                                                    e
                                                ));
                                            }
                                        };
                                    } else {
                                        self.last_error =
                                            Some(format!("Project not found: {}", pj.get_path()));
                                        self.display_buffer.log_error(format!(
                                            "Project not found: {}",
                                            pj.get_path()
                                        ));
                                        self.new_project.history.remove(i);
                                    }
                                }
                            }
                        });
                    });
                });

                let rect = ui.max_rect();
                let line_rect = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), rect.bottom() - 2.0),
                    egui::pos2(rect.right(), rect.bottom()),
                );
                ui.painter()
                    .rect_filled(line_rect, 0.0, uiutil::colors::ACCENT);
            });
    }

    /// Show the error display panel
    fn show_error_panel(&mut self, ctx: &egui::Context) {
        let should_show_error = self.last_error.is_some();
        if should_show_error {
            let error_text = self.last_error.clone().unwrap_or_default();
            egui::TopBottomPanel::bottom("error_panel")
                .frame(egui::Frame {
                    fill: uiutil::colors::ERROR,
                    inner_margin: 8.0.into(),
                    ..Default::default()
                })
                .show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!("[ERROR] {}", error_text))
                                .color(uiutil::colors::TEXT_PRIMARY),
                        );
                        if ui.add(uiutil::make_chip_button("Dismiss")).clicked() {
                            self.last_error = None;
                        }
                    });
                });
        }
    }
}

impl eframe::App for EvnApp {
    fn save(&mut self, storage: &mut dyn eframe::Storage) {
        eframe::set_value(storage, eframe::APP_KEY, self);
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.initialize();
        self.update_docker_status();
        self.show_top_panel(ctx);
        self.show_error_panel(ctx);
        self.show_docker_prompt(ctx);

        egui::CentralPanel::default().show(ctx, |ui| {
            // Project Creation Section
            let frame_project = uiutil::card_frame();
            frame_project.show(ui, |ui| {
                ProjectCreatePanel::show(ui, &mut self.new_project, &mut self.display_buffer);
            });

            ui.add_space(8.0);

            // DAP Server Section
            let frame_dap = uiutil::card_frame();
            frame_dap.show(ui, |ui| {
                DapServerPanel::show(ui, &mut self.probe_rs_dap_server, &mut self.display_buffer);
            });

            ui.add_space(8.0);

            // Logger Section
            let frame_logger = uiutil::card_frame_alt();
            frame_logger.show(ui, |ui| {
                LoggerPanel::show(ui, &mut self.display_buffer);
            });
        });

        if self.info {
            let _ = open::that(HELP_URL);
            self.info = false;
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
enum DockerStatus {
    Unknown,
    Running,
    Stopped,
}
