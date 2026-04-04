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
        document::Link { rel: "stylesheet", href: asset!("/assets/main.css") }

        div { class: "app-shell",

            // ===== TOP BAR =====
            div { class: "top-bar",

                div { class: "top-bar-left",
                    button {
                        class: "brand-icon brand-icon-button",
                        title: "Reset all settings",
                        onclick: move |_| show_reset_confirm.set(true),
                        "B"
                    }
                    span { class: "brand-name", "{parameter::APP_NAME}" }
                    span { class: "brand-version", "{parameter::build_version_label()}" }

                    div { class: "docker-status",
                        span { class: "{helpers::docker_dot_class(&docker_status.read())}" }
                        span { class: "docker-label", "{docker_status}" }
                    }
                }

                div { class: "top-bar-spacer" }

                div { class: "top-bar-right",

                    // History dropdown
                    div { class: "dropdown-container",
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
                                    div { class: "dropdown-empty", "No history yet" }
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
                                                div { class: "dropdown-item-name", "{entry_name}" }
                                                div { class: "dropdown-item-path", "{entry_path}" }
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
            main { class: "main-content",

                div { class: "cards-grid",

                    // ---- Create Project ----
                    section { class: "card",
                        h2 { class: "section-title", "Create Project" }
                        p { class: "section-subtitle",
                            "Generate a template-based project and open it in VS Code."
                        }

                        div { class: "card-body",
                            div { class: "input-row",
                                label { class: "input-label", "Project name" }
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

                            div { class: "input-row",
                                label { class: "input-label", "Version" }
                                div { class: "ref-type-toggle",
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

                            div { class: "checkbox-row",
                                input {
                                    r#type: "checkbox",
                                    checked: *vscode_open_enabled.read(),
                                    onchange: move |ev| {
                                        vscode_open_enabled.set(ev.checked());
                                    },
                                }
                                span { class: "checkbox-label", "Open VS Code after creation" }
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
                        div { class: "card-header-row",
                            div {
                                h2 { class: "section-title", "probe-rs DAP Server" }
                                p { class: "section-subtitle",
                                    "Launch a local DAP server for debugging."
                                }
                            }
                            div { class: "dap-status",
                                span { class: "{helpers::dap_dot_class(*dap_running.read())}" }
                                span {
                                    if *dap_running.read() {
                                        "Running"
                                    } else {
                                        "Stopped"
                                    }
                                }
                            }
                        }

                        div { class: "card-body",
                            div { class: "input-row",
                                label { class: "input-label", "Port" }
                                input {
                                    class: "input input-narrow",
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
                            p { class: "hint-text",
                                "Docker path mapping: set pathMappings in launch.json (remoteRoot / localRoot)."
                            }
                        }
                    }
                }

                // ---- Log ----
                section { class: "card card-full",
                    div { class: "card-header-row",
                        div {
                            h2 { class: "section-title", "Log" }
                            p { class: "section-subtitle", "Build and runtime output." }
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
                                span { class: "log-timestamp", "{logger::extract_timestamp(line)}" }
                                span { class: "log-badge {logger::log_badge_class(line)}",
                                    "{logger::extract_level(line)}"
                                }
                                span { class: "log-message", "{logger::extract_message(line)}" }
                            }
                        }
                    }
                }
            }

            // ---- Error toast ----
            if let Some(err) = last_error.read().clone() {
                div { class: "error-toast",
                    div { class: "error-toast-inner",
                        span { "[ERROR] {err}" }
                        button {
                            class: "error-dismiss",
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
                        h3 { class: "modal-title", "Rancher Desktop" }
                        p { class: "modal-text", "Docker is not running." }
                        p { class: "modal-text", "Start Rancher Desktop now?" }
                        div { class: "modal-actions",
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
                        h3 { class: "modal-title", "Reset Settings" }
                        p { class: "modal-text",
                            "All settings including project history will be cleared."
                        }
                        p { class: "modal-text",
                            "The application will restart from the splash screen."
                        }
                        div { class: "modal-actions",
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
                    p { class: "splash-subtitle", "{parameter::APP_NAME}" }
                    p { class: "splash-version", "{parameter::build_version_label()}" }
                }
            }
        }
    }
}
