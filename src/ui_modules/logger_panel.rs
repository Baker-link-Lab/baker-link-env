use crate::logger::DisplayBuffer;
use crate::uiutil;

/// UI for displaying logs
pub struct LoggerPanel;

impl LoggerPanel {
    pub fn show(ui: &mut egui::Ui, display_buffer: &mut DisplayBuffer) {
        ui.label(uiutil::make_section_title("Log"));
        ui.label(uiutil::make_section_subtitle("Build and runtime output."));
        ui.add_space(6.0);
        display_buffer.channel_recv();

        let text_style = egui::TextStyle::Body;
        let row_height = ui.text_style_height(&text_style);
        let num_rows = display_buffer.buffer.len();

        let log_frame = egui::Frame {
            inner_margin: 10.0.into(),
            rounding: 8.0.into(),
            fill: uiutil::colors::CARD_ALT,
            stroke: egui::Stroke::new(1.0, uiutil::colors::BORDER),
            ..Default::default()
        };

        log_frame.show(ui, |ui| {
            egui::ScrollArea::vertical().auto_shrink(false).show_rows(
                ui,
                row_height,
                num_rows,
                |ui, row_range| {
                    for i in row_range {
                        let log_text = &display_buffer.buffer[i];
                        let color = if log_text.contains("[ERROR]") {
                            uiutil::colors::ERROR
                        } else if log_text.contains("[DEBUG]") {
                            uiutil::colors::TEXT_MUTED
                        } else {
                            uiutil::colors::TEXT_PRIMARY
                        };

                        ui.label(
                            egui::RichText::new(log_text.clone())
                                .color(color)
                                .size(10.0),
                        );
                        ui.add_space(0.12);
                    }
                },
            );
        });

        ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
            powered_by_egui_and_eframe(ui);
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
