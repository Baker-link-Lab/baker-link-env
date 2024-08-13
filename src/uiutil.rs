pub fn set_background_color(ctx: &eframe::CreationContext, color: egui::Color32) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = color;
    ctx.egui_ctx.set_visuals(visuals);
}

pub fn set_fonts(ctx: &eframe::CreationContext) {
    let mut fonts = egui::FontDefinitions::default();

    fonts.font_data.insert(
        "my_font".to_owned(),
        egui::FontData::from_static(include_bytes!("../fonts/NotoSansJP-Bold.otf")),
    );

    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "my_font".to_owned());

    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("my_font".to_owned());

    ctx.egui_ctx.set_fonts(fonts);
}

pub fn get_frame() -> egui::Frame {
    egui::Frame {
        inner_margin: 12.0.into(),
        outer_margin: 5.0.into(),
        rounding: 12.0.into(),
        fill: egui::Color32::from_hex("#fff9ee").unwrap(),
        shadow: egui::Shadow {
            offset: [1.0, 1.0].into(),
            blur: 16.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(64),
        },
        ..Default::default()
    }
}

pub fn make_orange_button(text: &str) -> egui::Button {
    egui::Button::new(egui::RichText::new(text).color(egui::Color32::from_hex("#410002").unwrap()))
        .fill(egui::Color32::from_hex("#DD7032").unwrap())
}
