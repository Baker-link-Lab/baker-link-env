use dioxus::prelude::*;
use futures_util::StreamExt;
use std::time::Duration;

use crate::{cmd, helpers, logger, parameter, settings};

/// Actions dispatched from UI buttons into a single coroutine.
enum AppAction {
    StartDap,
    StopDap,
    StartDocker,
    OpenProject(String),
    DetectDevice,
}

#[component]
pub fn App() -> Element {
    // State signals
    let mut project_name = use_signal(|| "myproject".to_string());
    let mut template_ref_is_tag = use_signal(|| false); // false = branch, true = tag
    let mut template_ref_value = use_signal(|| String::new());
    let mut vscode_open_enabled = use_signal(|| true);
    let mut dap_port = use_signal(|| "50001".to_string());
    let mut dap_running = use_signal(|| false);
    let mut logs = use_signal(Vec::<String>::new);
    let mut docker_status = use_signal(|| "Docker: ?".to_string());
    let mut last_error = use_signal(|| Option::<String>::None);
    let mut docker_prompt_dismissed = use_signal(|| false);
    let mut history = use_signal(settings::load_history);
    let mut show_history = use_signal(|| false);
    let mut show_splash = use_signal(settings::should_show_splash);
    let mut show_reset_confirm = use_signal(|| false);
    let mut show_version_opts = use_signal(|| false);
    let mut probe_name = use_signal(|| String::new());
    let mut chip_name = use_signal(|| String::new());
    let mut chip_cores = use_signal(|| String::new());
    let mut chip_voltage = use_signal(|| String::new());
    let mut detecting = use_signal(|| false);

    // Action dispatcher coroutine — single place for all side-effects
    let actions = use_coroutine(move |mut rx: UnboundedReceiver<AppAction>| async move {
        while let Some(action) = rx.next().await {
            match action {
                AppAction::StartDap => {
                    if let Ok(mut server) = crate::dap_server().lock() {
                        let tx = {
                            if let Ok(buffer) = crate::display_buffer().lock() {
                                buffer.sender()
                            } else {
                                continue;
                            }
                        };
                        match server.start(tx) {
                            Ok(()) => {
                                dap_running.set(true);
                                crate::log_info(format!(
                                    "probe-rs DAP Server started on port {}",
                                    server.port
                                ));
                            }
                            Err(e) => {
                                crate::log_error(e.clone());
                                last_error.set(Some(e));
                            }
                        }
                    }
                }
                AppAction::StopDap => {
                    if let Ok(mut server) = crate::dap_server().lock() {
                        if server.stop() {
                            dap_running.set(false);
                            crate::log_info("probe-rs DAP Server stopped");
                        }
                    }
                }
                AppAction::StartDocker => {
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    std::thread::spawn(move || {
                        let _ = tx.send(cmd::start_rd());
                    });
                    match rx.await {
                        Ok(Ok(_)) => crate::log_info("Rancher Desktop started"),
                        Ok(Err(e)) => {
                            crate::log_error(format!("Rancher Desktop start failed: {}", e));
                            last_error.set(Some(e));
                        }
                        Err(_) => {
                            crate::log_error("Rancher Desktop start: channel closed");
                        }
                    }
                }
                AppAction::OpenProject(path) => {
                    if std::path::Path::new(&path).exists() {
                        // Start Rancher Desktop in background (non-blocking)
                        std::thread::spawn(|| {
                            let _ = cmd::start_rd();
                        });
                        crate::log_info(format!("Visual Studio Code opened: {}", path));
                        let path_clone = path.clone();
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        std::thread::spawn(move || {
                            let _ = tx.send(cmd::open_vscode(&path_clone));
                        });
                        if let Ok(Err(e)) = rx.await {
                            crate::log_error(format!("Visual Studio Code failed to open: {}", e));
                        }
                    } else {
                        crate::log_error(format!("Project not found: {}", path));
                        last_error.set(Some(format!("Project not found: {}", path)));
                        let mut hist = history.read().clone();
                        if let Some(pos) = hist.iter().position(|e| e.path == path) {
                            hist.remove(pos);
                            settings::save_history(&hist);
                            history.set(hist);
                        }
                    }
                }
                AppAction::DetectDevice => {
                    detecting.set(true);
                    probe_name.set(String::new());
                    chip_name.set(String::new());
                    chip_cores.set(String::new());
                    chip_voltage.set(String::new());
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    std::thread::spawn(move || {
                        let _ = tx.send(cmd::detect_target());
                    });
                    match rx.await {
                        Ok(Ok(info)) => {
                            probe_name.set(format!(
                                "{} ({})",
                                info.probe.identifier, info.probe.probe_type
                            ));
                            chip_name.set(info.chip_name.clone());
                            if !info.cores.is_empty() {
                                if info.cores.iter().all(|c| c == &info.cores[0]) {
                                    chip_cores.set(format!(
                                        "{} x{}",
                                        info.cores[0],
                                        info.cores.len()
                                    ));
                                } else {
                                    chip_cores.set(info.cores.join(", "));
                                }
                            }
                            if let Some(v) = info.target_voltage {
                                chip_voltage.set(format!("{v:.2}V"));
                            }
                            crate::log_info(format!(
                                "Detected: {} ({})",
                                info.chip_name, info.probe.identifier
                            ));
                        }
                        Ok(Err(e)) => {
                            let probes = cmd::list_probes();
                            if probes.is_empty() {
                                probe_name.set("No probe found".to_string());
                            } else {
                                let p = &probes[0];
                                probe_name.set(format!("{} ({})", p.identifier, p.probe_type));
                                chip_name.set("Detection failed".to_string());
                            }
                            crate::log_error(format!("Device detection: {e}"));
                        }
                        Err(_) => {
                            probe_name.set("Detection failed".to_string());
                        }
                    }
                    detecting.set(false);
                }
            }
        }
    });

    // Auto-dismiss splash after 3 seconds (re-triggers on reset)
    use_effect(move || {
        if *show_splash.read() {
            settings::mark_splash_shown();
            spawn(async move {
                tokio::time::sleep(Duration::from_millis(3000)).await;
                show_splash.set(false);
            });
        }
    });

    // Poll logs every 300ms
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_millis(300)).await;
            if let Ok(mut buffer) = crate::display_buffer().lock() {
                buffer.channel_recv();
                let latest = buffer.buffer.clone();
                if latest != *logs.read() {
                    logs.set(latest);
                }
            }
        }
    });

    // Docker status auto-polling every 5 seconds
    use_future(move || async move {
        loop {
            let (tx, rx) = tokio::sync::oneshot::channel();
            std::thread::spawn(move || {
                let _ = tx.send(cmd::is_docker_running());
            });
            if let Ok(result) = rx.await {
                let new_status = match result {
                    Ok(true) => "Docker: On".to_string(),
                    Ok(false) => "Docker: Off".to_string(),
                    Err(_) => "Docker: ?".to_string(),
                };
                if *docker_status.read() != new_status {
                    if new_status.contains("On") {
                        docker_prompt_dismissed.set(false);
                    }
                    docker_status.set(new_status);
                }
            }
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    });

    rsx! {
        document::Title { "{parameter::APP_NAME}" }
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        div { class: "app-shell",

            // ===== TOP BAR =====
            div { class: "flex items-center gap-4 py-2 px-5 border-b border-bkl-border bg-bkl-sidebar shrink-0",

                div { class: "flex items-center gap-2.5",
                    button {
                        class: "brand-icon brand-icon-button",
                        title: "Reset all settings",
                        onclick: move |_| show_reset_confirm.set(true),
                        "B"
                    }
                    span { class: "text-sm font-extrabold text-bkl-text leading-none tracking-tight",
                        "{parameter::APP_NAME}"
                    }
                    span { class: "text-[10px] text-bkl-text-faint ml-1.5",
                        "{parameter::build_version_label()}"
                    }

                    div { class: "flex items-center gap-1.5 ml-4 pl-4 border-l border-bkl-border",
                        span { class: "size-2 rounded-full inline-block shrink-0 {helpers::docker_dot_class(&docker_status.read())}" }
                        span { class: "text-xs text-bkl-text-muted", "{docker_status}" }
                    }
                }

                div { class: "flex-1" }

                div { class: "flex items-center gap-2",

                    // History dropdown
                    div { class: "relative",
                        button {
                            class: "btn-chip",
                            onclick: move |_| {
                                let current = *show_history.read();
                                show_history.set(!current);
                            },
                            "History"
                        }
                        if *show_history.read() {
                            div { class: "dropdown-menu",
                                if history.read().is_empty() {
                                    div { class: "p-4 text-center text-xs text-bkl-text-faint",
                                        "No history yet"
                                    }
                                }
                                for entry in history.read().iter() {
                                    {
                                        let entry_path = entry.path.clone();
                                        let entry_name = entry.name.clone();
                                        rsx! {
                                            button {
                                                class: "dropdown-item",
                                                onclick: move |_| {
                                                    show_history.set(false);
                                                    actions.send(AppAction::OpenProject(entry_path.clone()));
                                                },
                                                div { class: "text-[13px] font-semibold text-bkl-text", "{entry_name}" }
                                                div { class: "text-[11px] text-bkl-text-faint mt-0.5 break-all", "{entry_path}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }

                    button {
                        class: "btn-chip",
                        onclick: move |_| {
                            let _ = open::that(
                                "https://github.com/Baker-link-Lab/baker-link-env/blob/main/README.md",
                            );
                        },
                        "Help"
                    }
                }
            }

            // ===== MAIN CONTENT =====
            main { class: "main-content flex-1 flex flex-col overflow-hidden py-5 px-6",

                div { class: "grid grid-cols-2 gap-5 max-[860px]:grid-cols-1",

                    // ---- Create Project ----
                    section { class: "card",
                        h2 { class: "m-0 text-[15px] font-bold text-bkl-text tracking-[0.02em]",
                            "Create Project"
                        }
                        p { class: "mt-0.5 text-xs text-bkl-text-muted",
                            "Generate a template-based project and open it in VS Code."
                        }

                        div { class: "mt-4",
                            div { class: "flex flex-wrap items-center gap-2",
                                label { class: "text-[13px] font-semibold text-bkl-text-muted",
                                    "Project name"
                                }
                                input {
                                    class: "input",
                                    value: "{project_name}",
                                    oninput: move |ev| project_name.set(ev.value()),
                                }
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                            let path_str = path.to_string_lossy().to_string();
                                            let joined = std::path::Path::new(&path_str)
                                                .join(project_name.read().as_str());
                                            let joined_str = joined.to_string_lossy().to_string();
                                            if !joined.exists() {
                                                let ref_val = template_ref_value.read().trim().to_string();
                                                let ref_opt = if ref_val.is_empty() { None } else { Some(ref_val) };
                                                let (branch, tag) = if *template_ref_is_tag.read() {
                                                    (None, ref_opt)
                                                } else {
                                                    (ref_opt, None)
                                                };
                                                match cmd::generate_project(
                                                    project_name.read().as_str(),
                                                    &path_str,
                                                    branch,
                                                    tag,
                                                ) {
                                                    Ok(_) => {
                                                        crate::log_info(format!("Project {} generated", joined_str));
                                                        let mut h = history.read().clone();
                                                        if !h.iter().any(|e| e.path == joined_str) {
                                                            if h.len() >= settings::HISTORY_MAX {
                                                                h.remove(0);
                                                            }
                                                            h.push(settings::HistoryEntry {
                                                                name: project_name.read().clone(),
                                                                path: joined_str.clone(),
                                                            });
                                                            settings::save_history(&h);
                                                            history.set(h);
                                                        }
                                                    }
                                                    Err(e) => {
                                                        crate::log_error(format!("Project generation failed: {}", e));
                                                    }
                                                }
                                            } else {
                                                crate::log_info(format!("Project {} already exists", joined_str));
                                            }
                                            if *vscode_open_enabled.read() {
                                                let _ = cmd::start_rd();
                                                crate::log_info(format!("Visual Studio Code opened: {}", joined_str));
                                                if let Err(e) = cmd::open_vscode(&joined_str) {
                                                    crate::log_error(
                                                        format!("Visual Studio Code failed to open: {}", e),
                                                    );
                                                }
                                            }
                                        }
                                    },
                                    "Create"
                                }
                            }

                            div { class: "flex items-center gap-2 mt-2",
                                input {
                                    r#type: "checkbox",
                                    checked: *show_version_opts.read(),
                                    onchange: move |ev| {
                                        show_version_opts.set(ev.checked());
                                    },
                                }
                                span { class: "text-[13px] text-bkl-text-muted",
                                    "Specify template version"
                                }
                            }

                            if *show_version_opts.read() {
                                div { class: "flex flex-wrap items-center gap-2",
                                    label { class: "text-[13px] font-semibold text-bkl-text-muted",
                                        "Version"
                                    }
                                    div { class: "flex gap-1",
                                        button {
                                            class: if !*template_ref_is_tag.read() { "btn-chip btn-chip-active" } else { "btn-chip" },
                                            onclick: move |_| template_ref_is_tag.set(false),
                                            "Branch"
                                        }
                                        button {
                                            class: if *template_ref_is_tag.read() { "btn-chip btn-chip-active" } else { "btn-chip" },
                                            onclick: move |_| template_ref_is_tag.set(true),
                                            "Tag"
                                        }
                                    }
                                    input {
                                        class: "input",
                                        placeholder: if *template_ref_is_tag.read() { "e.g. v1.0.0 (blank = latest)" } else { "e.g. main (blank = HEAD)" },
                                        value: "{template_ref_value}",
                                        oninput: move |ev| template_ref_value.set(ev.value()),
                                    }
                                }
                            }

                            div { class: "flex items-center gap-2 mt-2",
                                input {
                                    r#type: "checkbox",
                                    checked: *vscode_open_enabled.read(),
                                    onchange: move |ev| {
                                        vscode_open_enabled.set(ev.checked());
                                    },
                                }
                                span { class: "text-[13px] text-bkl-text-muted",
                                    "Open VS Code after creation"
                                }
                            }

                            a {
                                class: "template-link",
                                href: "{parameter::TEMPLATE_URL}",
                                "View template repository"
                            }
                        }
                    }

                    // ---- DAP Server ----
                    section { class: "card",
                        div { class: "flex items-center justify-between",
                            div {
                                h2 { class: "m-0 text-[15px] font-bold text-bkl-text tracking-[0.02em]",
                                    "probe-rs DAP Server"
                                }
                                p { class: "mt-0.5 text-xs text-bkl-text-muted",
                                    "Launch a local DAP server for debugging."
                                }
                            }
                            div { class: "flex items-center gap-1.5 text-[11px] text-bkl-text-muted",
                                span { class: "size-2 rounded-full inline-block shrink-0 {helpers::dap_dot_class(*dap_running.read())}" }
                                span {
                                    if *dap_running.read() {
                                        "Running"
                                    } else {
                                        "Stopped"
                                    }
                                }
                            }
                        }

                        div { class: "mt-4",
                            div { class: "mb-3.5 pb-3.5 border-b border-bkl-border",
                                div { class: "flex items-center justify-between",
                                    span { class: "text-[13px] font-semibold text-bkl-text-muted",
                                        "Connected Device"
                                    }
                                    button {
                                        class: "btn-chip",
                                        disabled: *detecting.read(),
                                        onclick: move |_| actions.send(AppAction::DetectDevice),
                                        if *detecting.read() {
                                            "Detecting..."
                                        } else {
                                            "Detect"
                                        }
                                    }
                                }
                                if !probe_name.read().is_empty() {
                                    div { class: "probe-info-grid",
                                        div { class: "flex items-baseline gap-2 min-w-0",
                                            span { class: "text-[10px] font-bold text-bkl-text-faint uppercase tracking-[0.04em] shrink-0",
                                                "Probe"
                                            }
                                            span { class: "text-xs text-bkl-text-muted font-mono overflow-hidden text-ellipsis whitespace-nowrap",
                                                "{probe_name}"
                                            }
                                        }
                                        if !chip_name.read().is_empty() {
                                            div { class: "flex items-baseline gap-2 min-w-0",
                                                span { class: "text-[10px] font-bold text-bkl-text-faint uppercase tracking-[0.04em] shrink-0",
                                                    "Chip"
                                                }
                                                span { class: "text-xs font-mono font-semibold text-bkl-orange-light overflow-hidden text-ellipsis whitespace-nowrap",
                                                    "{chip_name}"
                                                }
                                            }
                                        }
                                        if !chip_cores.read().is_empty() {
                                            div { class: "flex items-baseline gap-2 min-w-0",
                                                span { class: "text-[10px] font-bold text-bkl-text-faint uppercase tracking-[0.04em] shrink-0",
                                                    "Core"
                                                }
                                                span { class: "text-xs text-bkl-text-muted font-mono overflow-hidden text-ellipsis whitespace-nowrap",
                                                    "{chip_cores}"
                                                }
                                            }
                                        }
                                        if !chip_voltage.read().is_empty() {
                                            div { class: "flex items-baseline gap-2 min-w-0",
                                                span { class: "text-[10px] font-bold text-bkl-text-faint uppercase tracking-[0.04em] shrink-0",
                                                    "Voltage"
                                                }
                                                span { class: "text-xs text-bkl-text-muted font-mono overflow-hidden text-ellipsis whitespace-nowrap",
                                                    "{chip_voltage}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "flex flex-wrap items-center gap-2",
                                label { class: "text-[13px] font-semibold text-bkl-text-muted",
                                    "Port"
                                }
                                input {
                                    class: "input min-w-[100px] w-[100px]",
                                    value: "{dap_port}",
                                    oninput: move |ev| {
                                        let value = ev.value();
                                        dap_port.set(value.clone());
                                        if let Ok(mut server) = crate::dap_server().lock() {
                                            server.port = value;
                                        }
                                    },
                                }
                                button {
                                    class: "btn-primary",
                                    disabled: *dap_running.read(),
                                    onclick: move |_| actions.send(AppAction::StartDap),
                                    "Run"
                                }
                                button {
                                    class: "btn-danger",
                                    disabled: !*dap_running.read(),
                                    onclick: move |_| actions.send(AppAction::StopDap),
                                    "Stop"
                                }
                            }
                            p { class: "mt-3 text-[11px] text-bkl-text-faint",
                                "Docker path mapping: set pathMappings in launch.json (remoteRoot / localRoot)."
                            }
                        }
                    }
                }

                // ---- Log ----
                section { class: "card mt-5 flex-1 flex flex-col min-h-0",
                    div { class: "flex items-center justify-between",
                        div {
                            h2 { class: "m-0 text-[15px] font-bold text-bkl-text tracking-[0.02em]",
                                "Log"
                            }
                            p { class: "mt-0.5 text-xs text-bkl-text-muted",
                                "Build and runtime output."
                            }
                        }
                        button {
                            class: "btn-chip",
                            onclick: move |_| {
                                let text = logs.read().join("\n");
                                if let Ok(mut cb) = arboard::Clipboard::new() {
                                    let _ = cb.set_text(text);
                                }
                            },
                            "Copy"
                        }
                    }
                    div { class: "log-viewer",
                        for (idx , line) in logs.read().iter().enumerate() {
                            div {
                                key: "{idx}",
                                class: "log-line {logger::log_level_class(line)}",
                                span { class: "text-bkl-text-faint shrink-0 select-all",
                                    "{logger::extract_timestamp(line)}"
                                }
                                span { class: "log-badge {logger::log_badge_class(line)}",
                                    "{logger::extract_level(line)}"
                                }
                                span { class: "break-all", "{logger::extract_message(line)}" }
                            }
                        }
                    }
                }
            }

            // ---- Error toast ----
            if let Some(err) = last_error.read().clone() {
                div { class: "error-toast",
                    div { class: "flex items-start justify-between gap-3",
                        span { "[ERROR] {err}" }
                        button {
                            class: "shrink-0 text-bkl-text-faint text-lg leading-none cursor-pointer bg-transparent border-none p-0 hover:text-bkl-red",
                            onclick: move |_| last_error.set(None),
                            "\u{00d7}"
                        }
                    }
                }
            }

            // ---- Docker startup prompt ----
            if docker_status.read().contains("Off") && !*docker_prompt_dismissed.read()
                && !*show_splash.read()
            {
                div { class: "modal-overlay",
                    div { class: "modal",
                        h3 { class: "m-0 mb-3 text-base font-bold text-bkl-text", "Rancher Desktop" }
                        p { class: "m-0 mb-1 text-[13px] text-bkl-text-muted",
                            "Docker is not running."
                        }
                        p { class: "m-0 mb-1 text-[13px] text-bkl-text-muted",
                            "Start Rancher Desktop now?"
                        }
                        div { class: "flex gap-2 mt-5",
                            button {
                                class: "btn-primary",
                                onclick: move |_| {
                                    actions.send(AppAction::StartDocker);
                                    docker_prompt_dismissed.set(true);
                                },
                                "Start"
                            }
                            button {
                                class: "btn-chip",
                                onclick: move |_| docker_prompt_dismissed.set(true),
                                "Not now"
                            }
                        }
                    }
                }
            }

            // ---- Reset confirmation modal ----
            if *show_reset_confirm.read() {
                div { class: "modal-overlay",
                    div { class: "modal",
                        h3 { class: "m-0 mb-3 text-base font-bold text-bkl-text", "Reset Settings" }
                        p { class: "m-0 mb-1 text-[13px] text-bkl-text-muted",
                            "All settings including project history will be cleared."
                        }
                        p { class: "m-0 mb-1 text-[13px] text-bkl-text-muted",
                            "The application will restart from the splash screen."
                        }
                        div { class: "flex gap-2 mt-5",
                            button {
                                class: "btn-danger",
                                onclick: move |_| {
                                    settings::reset_all();
                                    history.set(vec![]);
                                    show_reset_confirm.set(false);
                                    show_splash.set(true);
                                },
                                "Reset"
                            }
                            button {
                                class: "btn-chip",
                                onclick: move |_| show_reset_confirm.set(false),
                                "Cancel"
                            }
                        }
                    }
                }
            }

            // ---- Splash screen ----
            if *show_splash.read() {
                div {
                    class: "splash-overlay",
                    onclick: move |_| show_splash.set(false),
                    div { class: "splash-glow" }
                    div { class: "splash-glow splash-glow-2" }
                    div { class: "splash-content",
                        img {
                            class: "splash-logo",
                            src: "{helpers::logo_data_uri()}",
                            alt: "Baker Link",
                        }
                        div { class: "splash-shimmer" }
                    }
                    p { class: "mt-5 text-[13px] font-semibold text-bkl-text-muted tracking-[0.08em] animate-splash-text-1",
                        "{parameter::APP_NAME}"
                    }
                    p { class: "mt-1.5 text-[11px] text-bkl-text-faint animate-splash-text-2",
                        "{parameter::build_version_label()}"
                    }
                }
            }
        }
    }
}
