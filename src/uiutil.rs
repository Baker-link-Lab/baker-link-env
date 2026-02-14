// Color Palette - Cream Base, Warm Orange Accent
// A soft cream canvas with warm orange highlights.

pub mod colors {
    use egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(0xf7, 0xf0, 0xe6);
    pub const BG_TOP: Color32 = Color32::from_rgb(0xf4, 0xe9, 0xdb);
    pub const CARD: Color32 = Color32::from_rgb(0xff, 0xf8, 0xee);
    pub const CARD_ALT: Color32 = Color32::from_rgb(0xf2, 0xe5, 0xd4);
    pub const CARD_HOVER: Color32 = Color32::from_rgb(0xea, 0xd8, 0xc3);
    pub const BORDER: Color32 = Color32::from_rgb(0xd9, 0xc6, 0xb0);

    pub const ACCENT: Color32 = Color32::from_rgb(0xe0, 0x7a, 0x1f);
    pub const ACCENT_DARK: Color32 = Color32::from_rgb(0xb8, 0x5d, 0x16);
    pub const ERROR: Color32 = Color32::from_rgb(0xd6, 0x45, 0x2b);

    pub const TEXT_PRIMARY: Color32 = Color32::from_rgb(0x3b, 0x2b, 0x1f);
    pub const TEXT_SECONDARY: Color32 = Color32::from_rgb(0x6b, 0x4f, 0x3a);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0x8c, 0x71, 0x5d);
    pub const TEXT_ON_ACCENT: Color32 = Color32::from_rgb(0xff, 0xf8, 0xee);

    pub const STATUS_ON: Color32 = Color32::from_rgb(0x3b, 0xa7, 0x55);
    pub const STATUS_OFF: Color32 = Color32::from_rgb(0xc6, 0xb5, 0xa4);
    pub const STATUS_UNKNOWN: Color32 = Color32::from_rgb(0xe0, 0x9f, 0x3a);
}

pub fn apply_theme(ctx: &eframe::CreationContext) {
    let mut visuals = egui::Visuals::light();
    visuals.panel_fill = colors::BG;
    visuals.window_fill = colors::CARD;
    visuals.faint_bg_color = colors::CARD_ALT;
    visuals.extreme_bg_color = colors::BG_TOP;
    visuals.window_rounding = 14.0.into();
    visuals.popup_shadow = egui::Shadow {
        offset: [0.0, 6.0].into(),
        blur: 20.0,
        spread: 0.0,
        color: egui::Color32::from_black_alpha(60),
    };

    visuals.widgets.inactive.bg_fill = colors::CARD;
    visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, colors::BORDER);

    visuals.widgets.hovered.bg_fill = colors::CARD_HOVER;
    visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);
    visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, colors::ACCENT);

    visuals.widgets.active.bg_fill = colors::ACCENT;
    visuals.widgets.active.fg_stroke = egui::Stroke::new(1.0, colors::TEXT_ON_ACCENT);
    visuals.widgets.active.bg_stroke = egui::Stroke::new(1.0, colors::ACCENT);

    visuals.selection.bg_fill = colors::ACCENT;
    visuals.selection.stroke = egui::Stroke::new(1.0, colors::TEXT_PRIMARY);

    let mut style = (*ctx.egui_ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.spacing.button_padding = egui::vec2(12.0, 7.0);
    style.spacing.window_margin = egui::Margin::same(8.0);
    ctx.egui_ctx.set_style(style);
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

pub fn card_frame() -> egui::Frame {
    egui::Frame {
        inner_margin: 14.0.into(),
        outer_margin: 6.0.into(),
        rounding: 14.0.into(),
        fill: colors::CARD,
        stroke: egui::Stroke::new(1.0, colors::BORDER),
        shadow: egui::Shadow {
            offset: [0.0, 8.0].into(),
            blur: 24.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(60),
        },
        ..Default::default()
    }
}

pub fn card_frame_alt() -> egui::Frame {
    egui::Frame {
        inner_margin: 14.0.into(),
        outer_margin: 6.0.into(),
        rounding: 14.0.into(),
        fill: colors::CARD_ALT,
        stroke: egui::Stroke::new(1.0, colors::BORDER),
        shadow: egui::Shadow {
            offset: [0.0, 8.0].into(),
            blur: 24.0,
            spread: 0.0,
            color: egui::Color32::from_black_alpha(60),
        },
        ..Default::default()
    }
}

pub fn header_frame() -> egui::Frame {
    egui::Frame {
        inner_margin: egui::Margin::symmetric(14.0, 10.0),
        outer_margin: egui::Margin::same(0.0),
        rounding: 0.0.into(),
        fill: colors::BG_TOP,
        ..Default::default()
    }
}

pub fn make_primary_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(
        egui::RichText::new(text)
            .color(colors::TEXT_ON_ACCENT)
            .size(12.0),
    )
    .fill(colors::ACCENT)
    .stroke(egui::Stroke::new(1.0, colors::ACCENT_DARK))
}

pub fn make_danger_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(
        egui::RichText::new(text)
            .color(colors::TEXT_ON_ACCENT)
            .size(12.0),
    )
    .fill(colors::ERROR)
    .stroke(egui::Stroke::new(1.0, colors::ACCENT_DARK))
}

pub fn make_chip_button(text: &str) -> egui::Button<'_> {
    egui::Button::new(egui::RichText::new(text).color(colors::TEXT_PRIMARY))
        .fill(colors::CARD)
        .stroke(egui::Stroke::new(1.0, colors::BORDER))
}

pub fn make_primary_heading(text: impl Into<String>) -> egui::RichText {
    egui::RichText::new(text)
        .size(28.0)
        .color(colors::TEXT_PRIMARY)
        .strong()
}

pub fn make_section_title(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(16.0)
        .color(colors::TEXT_PRIMARY)
        .strong()
}

pub fn make_section_subtitle(text: &str) -> egui::RichText {
    egui::RichText::new(text)
        .size(11.0)
        .color(colors::TEXT_SECONDARY)
}

pub fn status_dot(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _resp) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}
