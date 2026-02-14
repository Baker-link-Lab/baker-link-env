use crate::cmd;
use crate::logger::DisplayBuffer;
use crate::uiutil;

const TEMPLATE_URL: &str = "https://github.com/Baker-link-Lab/bakerlink_tutorial_template";

/// UI for creating new projects
pub struct ProjectCreatePanel;

impl ProjectCreatePanel {
    pub fn show(
        ui: &mut egui::Ui,
        new_project: &mut cmd::NewProject,
        display_buffer: &mut DisplayBuffer,
    ) {
        ui.label(uiutil::make_section_title("Create Project"));
        ui.label(uiutil::make_section_subtitle(
            "Generate a template-based project and open it in VS Code.",
        ));
        ui.add_space(6.0);

        ui.horizontal(|ui| {
            ui.label("Project name");
            ui.add(
                egui::TextEdit::singleline(&mut new_project.name)
                    .desired_width(180.0)
                    .hint_text("my-project"),
            );
            if ui.add(uiutil::make_primary_button("Create")).clicked() {
                if let Some(path) = rfd::FileDialog::new().pick_folder() {
                    new_project.path = path.to_string_lossy().to_string();
                    let join_path = std::path::Path::new(&new_project.path).join(&new_project.name);
                    if !join_path.exists() {
                        match cmd::generate_project(&new_project.name, &new_project.path) {
                            Ok(_) => {
                                new_project.history_push();
                                display_buffer.log_info(format!(
                                    "Project {} generated",
                                    join_path.to_str().unwrap(),
                                ));
                            }
                            Err(e) => {
                                display_buffer.log_error(format!(
                                    "Project {} generation failed: {}",
                                    join_path.to_str().unwrap(),
                                    e,
                                ));
                            }
                        }
                    } else {
                        display_buffer.log_info(format!(
                            "Project {} already exists",
                            join_path.to_str().unwrap(),
                        ));
                    }
                    if new_project.vscode_open_enabled {
                        let _ = cmd::start_rd();
                        match cmd::open_vscode(join_path.to_str().unwrap()) {
                            Ok(_) => {
                                display_buffer.log_info(format!(
                                    "Visual Studio Code opened: {}",
                                    join_path.to_str().unwrap()
                                ));
                            }
                            Err(e) => {
                                display_buffer.log_error(format!(
                                    "Visual Studio Code failed to open: {}: {}",
                                    join_path.to_str().unwrap(),
                                    e,
                                ));
                            }
                        };
                    }
                }
            }
        });
        ui.add_space(4.0);
        ui.checkbox(
            &mut new_project.vscode_open_enabled,
            "Open VS Code after creation",
        );
        ui.hyperlink_to("View template repository", TEMPLATE_URL);
    }
}
