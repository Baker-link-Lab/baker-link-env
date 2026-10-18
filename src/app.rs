use dioxus::prelude::*;
use futures_util::StreamExt;
use std::time::Duration;

use crate::{artifact, cmd, doctor, helpers, i18n::Language, logger, parameter, settings};

enum CacheAction {
    Refresh,
    Save(String),
    ClearUnused,
}

/// Actions dispatched from UI buttons into a single coroutine.
enum AppAction {
    StartDap(settings::ServerConfig),
    StopDap,
    StartDocker,
    CreateProject {
        name: String,
        parent_path: String,
        branch: Option<String>,
        tag: Option<String>,
        open_after: bool,
    },
    OpenProject(String),
    DetectDevice,
    DiagnoseEnvironment {
        startup: bool,
    },
    OpenHelp(Option<&'static str>),
    Cache(CacheAction),
}

fn localized_log_line(line: &str, language: Language) -> String {
    let display = logger::present_log_line(line);
    if let Some((prefix, _)) = line.split_once("]: ") {
        format!("{}]: {}", prefix, language.text(display.message))
    } else {
        language.text(line).into_owned()
    }
}

#[component]
pub fn App() -> Element {
    let server_config = settings::server_config().unwrap_or_else(|error| {
        crate::log_error(format!("Failed to load server settings: {error}"));
        settings::ServerConfig::default()
    });
    // State signals
    let mut project_name = use_signal(|| "myproject".to_string());
    let mut template_ref_is_tag = use_signal(|| false); // false = branch, true = tag
    let mut template_ref_value = use_signal(String::new);
    let mut vscode_open_enabled = use_signal(|| true);
    let mut dap_ip = use_signal(|| server_config.bind_ip);
    let mut dap_port = use_signal(|| server_config.dap_port);
    let mut upload_port = use_signal(|| server_config.upload_port);
    let mut dap_running = use_signal(|| false);
    let mut dap_stopping = use_signal(|| false);
    let mut logs = use_signal(Vec::<String>::new);
    let mut docker_status = use_signal(|| "Docker: ?".to_string());
    let mut last_error = use_signal(|| Option::<String>::None);
    let mut docker_prompt_dismissed = use_signal(|| false);
    let mut history = use_signal(|| {
        settings::load_history().unwrap_or_else(|error| {
            crate::log_error(format!("Failed to load project history: {error}"));
            Vec::new()
        })
    });
    let mut show_history = use_signal(|| false);
    let mut show_splash = use_signal(|| {
        settings::should_show_splash().unwrap_or_else(|error| {
            crate::log_error(format!("Failed to load splash state: {error}"));
            false
        })
    });
    let mut show_reset_confirm = use_signal(|| false);
    let mut show_version_opts = use_signal(|| false);
    let mut show_dap_advanced = use_signal(|| false);
    let mut cache_usage = use_signal(|| Option::<artifact::Usage>::None);
    let mut cache_limit_mib = use_signal(|| "1024".to_string());
    let mut cache_busy = use_signal(|| false);
    let mut probe_name = use_signal(String::new);
    let mut chip_name = use_signal(String::new);
    let mut chip_cores = use_signal(String::new);
    let mut detecting = use_signal(|| false);
    let mut show_doctor = use_signal(|| false);
    let mut diagnosing = use_signal(|| false);
    let mut doctor_report = use_signal(|| Option::<doctor::Report>::None);
    let mut startup_diagnosis_pending = use_signal(|| true);
    let mut doctor_notice = use_signal(|| Option::<String>::None);
    let mut language = use_signal(|| {
        settings::load()
            .map(|settings| settings.language)
            .unwrap_or_else(|error| {
                crate::log_error(format!("Failed to load language settings: {error}"));
                Language::English
            })
    });

    // Action dispatcher coroutine — single place for all side-effects
    let actions = use_coroutine(move |mut rx: UnboundedReceiver<AppAction>| async move {
        while let Some(action) = rx.next().await {
            match action {
                AppAction::Cache(action) => {
                    cache_busy.set(true);
                    let (tx, done) = tokio::sync::oneshot::channel();
                    std::thread::spawn(move || {
                        let result = (|| -> anyhow::Result<artifact::Usage> {
                            match action {
                                CacheAction::Refresh => {
                                    let limit = settings::cache_limit_bytes()?;
                                    artifact::configure_limit(limit)
                                }
                                CacheAction::ClearUnused => artifact::clear_unused(),
                                CacheAction::Save(value) => {
                                    let mib = value.trim().parse::<u64>().map_err(|_| {
                                        anyhow::anyhow!(
                                            "Cache capacity must be a whole number of MiB"
                                        )
                                    })?;
                                    let bytes = settings::cache_limit_from_mib(mib)?;
                                    let previous = settings::cache_limit_bytes()?;
                                    let usage = artifact::configure_limit(bytes)?;
                                    if let Err(error) = settings::save_cache_limit(mib) {
                                        if let Err(restore_error) =
                                            artifact::configure_limit(previous)
                                        {
                                            anyhow::bail!("Failed to save cache capacity: {error:#}; failed to restore previous capacity: {restore_error:#}");
                                        }
                                        return Err(error);
                                    }
                                    Ok(usage)
                                }
                            }
                        })();
                        let _ = tx.send(result);
                    });
                    match done.await {
                        Ok(Ok(usage)) => {
                            cache_limit_mib.set((usage.limit / (1024 * 1024)).to_string());
                            cache_usage.set(Some(usage));
                            crate::log_info("Artifact cache updated");
                        }
                        result => {
                            let message = match result {
                                Ok(Err(error)) => {
                                    format!("Failed to manage artifact cache: {error:#}")
                                }
                                Err(error) => format!("Artifact cache worker failed: {error}"),
                                _ => unreachable!(),
                            };
                            crate::log_error(message.clone());
                            last_error.set(Some(message));
                        }
                    }
                    cache_busy.set(false);
                }
                AppAction::OpenHelp(section) => {
                    let url = language.read().help_url(section);
                    if let Err(error) = open::that(&url) {
                        let message =
                            format!("Failed to open Help: {error}. Open {url} in your browser.");
                        crate::log_error(message.clone());
                        last_error.set(Some(message));
                    }
                }
                AppAction::DiagnoseEnvironment { startup } => {
                    if *diagnosing.read() {
                        continue;
                    }
                    diagnosing.set(true);
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    std::thread::spawn(move || {
                        let _ = tx.send(doctor::run());
                    });
                    match rx.await {
                        Ok(report) => {
                            for check in &report.checks {
                                if check.status == doctor::Status::NeedsAttention {
                                    crate::log_error(format!(
                                        "Doctor / {}: {}\n{}",
                                        check.name, check.message, check.details
                                    ));
                                }
                            }
                            crate::log_info(format!("Doctor: {}", report.summary()));
                            if startup && !report.ready() {
                                show_doctor.set(true);
                                show_history.set(false);
                                docker_prompt_dismissed.set(true);
                                doctor_notice.set(Some(
                                    "Some environment checks need your attention. Open the setup steps for the failed item in Doctor."
                                        .to_string(),
                                ));
                            }
                            doctor_report.set(Some(report));
                        }
                        Err(error) => {
                            let message = format!("Environment diagnosis failed: {error}");
                            crate::log_error(message.clone());
                            last_error.set(Some(message));
                            if startup {
                                show_doctor.set(true);
                                docker_prompt_dismissed.set(true);
                            }
                        }
                    }
                    diagnosing.set(false);
                    if startup {
                        startup_diagnosis_pending.set(false);
                    }
                }
                AppAction::StartDap(config) => {
                    if let Err(error) = settings::save_server_config(&config) {
                        let message = format!("Failed to save server settings: {error}");
                        crate::log_error(message.clone());
                        last_error.set(Some(message));
                        continue;
                    }
                    if let Ok(mut server) = crate::dap_server().lock() {
                        server.ip = config.bind_ip.clone();
                        server.port = config.dap_port.clone();
                        server.upload_port = config.upload_port.clone();
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
                                dap_stopping.set(false);
                                crate::log_info(format!(
                                    "probe-rs DAP Server started on {}:{}",
                                    server.ip, server.port
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
                            dap_stopping.set(true);
                            crate::log_info("Stopping probe-rs DAP and ELF upload servers...");
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
                AppAction::CreateProject {
                    name,
                    parent_path,
                    branch,
                    tag,
                    open_after,
                } => {
                    let project_path = std::path::Path::new(&parent_path).join(&name);
                    let project_path = project_path.to_string_lossy().to_string();
                    let already_exists = std::path::Path::new(&project_path).exists();
                    let worker_path = project_path.clone();
                    let worker_name = name.clone();
                    let (tx, rx) = tokio::sync::oneshot::channel();
                    std::thread::spawn(move || {
                        let result = if already_exists {
                            Ok(false)
                        } else {
                            cmd::generate_project(&worker_name, &parent_path, branch, tag)
                                .map(|_| true)
                                .map_err(|error| error.to_string())
                        };
                        let result = result.map(|generated| {
                            let open_error = if open_after {
                                cmd::open_vscode(&worker_path)
                                    .err()
                                    .map(|error| format!("{error:#}"))
                            } else {
                                None
                            };
                            (generated, open_error)
                        });
                        let _ = tx.send(result);
                    });

                    match rx.await {
                        Ok(Ok((generated, open_error))) => {
                            if generated {
                                crate::log_info(format!("Project {project_path} generated"));
                                let mut entries = history.read().clone();
                                if !entries.iter().any(|entry| entry.path == project_path) {
                                    if entries.len() >= settings::HISTORY_MAX {
                                        entries.remove(0);
                                    }
                                    entries.push(settings::HistoryEntry {
                                        name,
                                        path: project_path.clone(),
                                    });
                                    if let Err(error) = settings::save_history(&entries) {
                                        crate::log_error(format!(
                                            "Failed to save project history: {error}"
                                        ));
                                    }
                                    history.set(entries);
                                }
                            } else {
                                crate::log_info(format!("Project {project_path} already exists"));
                            }
                            if open_after && open_error.is_none() {
                                crate::log_info(format!(
                                    "Visual Studio Code open requested: {project_path}"
                                ));
                            }
                            if let Some(error) = open_error {
                                crate::log_error(format!(
                                    "Visual Studio Code failed to open: {error}"
                                ));
                                last_error.set(Some(error));
                            }
                        }
                        Ok(Err(error)) => {
                            crate::log_error(format!("Project generation failed: {error}"));
                            last_error.set(Some(error));
                        }
                        Err(error) => {
                            let message = format!("Project creation worker failed: {error}");
                            crate::log_error(message.clone());
                            last_error.set(Some(message));
                        }
                    }
                }
                AppAction::OpenProject(path) => {
                    if std::path::Path::new(&path).exists() {
                        let path_clone = path.clone();
                        let (tx, rx) = tokio::sync::oneshot::channel();
                        std::thread::spawn(move || {
                            let _ = tx.send(cmd::open_vscode(&path_clone));
                        });
                        match rx.await {
                            Ok(Ok(())) => crate::log_info(format!(
                                "Visual Studio Code open requested: {path}"
                            )),
                            Ok(Err(error)) => {
                                let message =
                                    format!("Visual Studio Code failed to open: {error:#}");
                                crate::log_error(message.clone());
                                last_error.set(Some(message));
                            }
                            Err(error) => {
                                let message = format!("VS Code worker failed: {error}");
                                crate::log_error(message.clone());
                                last_error.set(Some(message));
                            }
                        }
                    } else {
                        crate::log_error(format!("Project not found: {}", path));
                        last_error.set(Some(format!("Project not found: {}", path)));
                        let mut hist = history.read().clone();
                        if let Some(pos) = hist.iter().position(|e| e.path == path) {
                            hist.remove(pos);
                            if let Err(error) = settings::save_history(&hist) {
                                crate::log_error(format!(
                                    "Failed to save project history: {error}"
                                ));
                            }
                            history.set(hist);
                        }
                    }
                }
                AppAction::DetectDevice => {
                    detecting.set(true);
                    probe_name.set(String::new());
                    chip_name.set(String::new());
                    chip_cores.set(String::new());
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

    use_future(move || async move {
        actions.send(AppAction::Cache(CacheAction::Refresh));
        actions.send(AppAction::DiagnoseEnvironment { startup: true });
    });

    // Auto-dismiss splash after 3 seconds (re-triggers on reset)
    use_effect(move || {
        if *show_splash.read() {
            if let Err(error) = settings::mark_splash_shown() {
                crate::log_error(format!("Failed to save splash state: {error}"));
            }
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
            if let Ok(mut server) = crate::dap_server().lock() {
                let status = server.refresh_status();
                if status == cmd::DapServerStatus::Stopped && *dap_stopping.read() {
                    dap_stopping.set(false);
                    crate::log_info("probe-rs DAP and ELF upload servers stopped");
                }
                if status == cmd::DapServerStatus::Stopped && *dap_running.read() {
                    dap_running.set(false);
                }
            }
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

        div { class: "app-shell",

            // ===== TOP BAR =====
            div { class: "flex items-center gap-4 py-2 px-5 border-b border-bkl-border bg-bkl-sidebar shrink-0",
                style: "flex-wrap: wrap;",

                div { class: "flex items-center gap-2.5",
                    button {
                        class: "brand-icon brand-icon-button",
                        title: language.read().text("Reset all settings").into_owned(),
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
                        span { class: "text-xs text-bkl-text-muted", {language.read().text(&docker_status.read()).into_owned()} }
                    }
                }

                div { class: "flex-1" }

                div { class: "flex items-center gap-2",

                    button {
                        class: "btn-chip",
                        onclick: move |_| {
                            let current = *show_doctor.peek();
                            show_doctor.set(!current);
                            show_history.set(false);
                            docker_prompt_dismissed.set(true);
                        },
                        if *show_doctor.read() { {language.read().text("Home").into_owned()} } else { {language.read().text("Doctor").into_owned()} }
                    }

                    // History dropdown
                    div { class: "relative",
                        button {
                            class: "btn-chip",
                            onclick: move |_| {
                                let current = *show_history.read();
                                show_history.set(!current);
                            },
                            {language.read().text("History").into_owned()}
                        }
                        if *show_history.read() {
                            div { class: "dropdown-menu",
                                if history.read().is_empty() {
                                    div { class: "p-4 text-center text-xs text-bkl-text-faint",
                                        {language.read().text("No history yet").into_owned()}
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

                    label {
                        r#for: "display-language",
                        class: "text-xs text-bkl-text-muted",
                        {language.read().text("Language").into_owned()}
                    }
                    select {
                        id: "display-language",
                        class: "input",
                        aria_label: language.read().text("Language").into_owned(),
                        title: language.read().text("Language").into_owned(),
                        value: if *language.read() == Language::Japanese { "ja" } else { "en" },
                        onchange: move |event| {
                            let selected = if event.value() == "ja" {
                                Language::Japanese
                            } else {
                                Language::English
                            };
                            match settings::save_language(selected) {
                                Ok(()) => language.set(selected),
                                Err(error) => {
                                    let message = format!("Failed to save language settings: {error}");
                                    crate::log_error(message.clone());
                                    last_error.set(Some(message));
                                }
                            }
                        },
                        option { value: "en", "English" }
                        option { value: "ja", "日本語" }
                    }
                    button {
                        class: "btn-chip",
                        onclick: move |_| actions.send(AppAction::OpenHelp(None)),
                        {language.read().text("Help").into_owned()}
                    }
                }
            }

            if *show_doctor.read() {
                main {
                    class: "main-content flex-1 py-5 px-6",
                    style: "overflow-y: auto;",
                    button {
                        class: "btn-chip",
                        title: language.read().text("Return to Home").into_owned(),
                        onclick: move |_| show_doctor.set(false),
                        {language.read().text("\u{2190} Back to Home").into_owned()}
                    }
                    div { class: "flex items-center justify-between gap-3",
                        div {
                            h1 { class: "m-0 text-[15px] font-bold text-bkl-text", {language.read().text("Doctor").into_owned()} }
                            p { class: "text-xs text-bkl-text-muted",
                                {language.read().text("Check your development tools and Docker runtime. No projects or settings are changed.").into_owned()}
                            }
                        }
                        button {
                            class: "btn-primary",
                            disabled: *diagnosing.read(),
                            onclick: move |_| actions.send(AppAction::DiagnoseEnvironment { startup: false }),
                            if *diagnosing.read() { {language.read().text("Checking...").into_owned()} } else { {language.read().text("Run checks").into_owned()} }
                        }
                    }
                    if *diagnosing.read() {
                        p { role: "status", class: "text-xs text-bkl-text-muted",
                            {language.read().text("Checking VS Code, Dev Containers, and Docker. Each command has a 15-second timeout.").into_owned()}
                        }
                    }
                    if let Some(report) = doctor_report.read().clone() {
                        section { class: "card mt-5",
                            p { role: "status", class: "text-[13px] font-semibold text-bkl-text",
                                if *diagnosing.read() { {language.read().text("Previous result (checks are running)").into_owned()} } else { {language.read().text(report.summary()).into_owned()} }
                            }
                            p { class: "text-xs text-bkl-text-faint",
                                {language.read().text(&format!("Last checked: {}", report.checked_at)).into_owned()}
                            }
                            p { class: "text-xs text-bkl-text-muted",
                                {language.read().text("This checks host environment readiness only, not project configuration, container connections, or USB devices.").into_owned()}
                            }
                            if !report.ready() {
                                p { class: "text-xs text-bkl-text-muted",
                                    {language.read().text("Start with a Needs attention item and click Read setup steps. Help opens at the matching section in your browser. Follow the steps, return here, and click Run checks.").into_owned()}
                                }
                                button {
                                    class: "btn-chip",
                                    onclick: move |_| actions.send(AppAction::OpenHelp(Some("environment-setup"))),
                                    {language.read().text("Start here: environment setup").into_owned()}
                                }
                            }
                            button {
                                class: "btn-chip",
                                onclick: move |_| {
                                    if let Some(report) = doctor_report.read().as_ref() {
                                        let result = arboard::Clipboard::new()
                                            .and_then(|mut clipboard| clipboard.set_text(report.localized_text(*language.read())));
                                        if let Err(error) = result {
                                            let message = format!("Failed to copy Doctor report: {error}");
                                            crate::log_error(message.clone());
                                            last_error.set(Some(message));
                                        }
                                    }
                                },
                                {language.read().text("Copy report").into_owned()}
                            }
                        }
                        for check in report.checks {
                            section { class: "card mt-5",
                                div { class: "flex items-center justify-between gap-3",
                                    h2 { class: "m-0 text-[13px] font-bold text-bkl-text", {language.read().text(check.name).into_owned()} }
                                    span {
                                        class: "text-xs",
                                        style: match check.status {
                                            doctor::Status::Passed => "color: #3dd68c;",
                                            doctor::Status::NeedsAttention => "color: #ef4444;",
                                            doctor::Status::NotChecked => "color: #8a8494;",
                                        },
                                        {language.read().text(check.status.label()).into_owned()}
                                    }
                                }
                                p { class: "text-xs text-bkl-text-muted", {language.read().text(&check.message).into_owned()} }
                                if !check.details.is_empty() {
                                    details {
                                        summary { class: "text-xs text-bkl-text-muted", {language.read().text("Details").into_owned()} }
                                        pre { class: "text-xs text-bkl-text-muted", style: "white-space: pre-wrap; overflow-wrap: anywhere;", {language.read().text(&check.details).into_owned()} }
                                    }
                                }
                                if let Some(section) = check.help_section {
                                    button {
                                        class: "btn-chip",
                                        onclick: move |_| actions.send(AppAction::OpenHelp(Some(section))),
                                        {language.read().text("Read setup steps").into_owned()}
                                    }
                                }
                            }
                        }
                    } else {
                        section { class: "card mt-5",
                            p { class: "text-xs text-bkl-text-muted",
                                {language.read().text("Run checks to see what is ready and what needs attention. Docker does not need to be running to start the diagnosis.").into_owned()}
                            }
                            button {
                                class: "btn-chip",
                                onclick: move |_| actions.send(AppAction::OpenHelp(Some("environment-setup"))),
                                {language.read().text("Start here: environment setup").into_owned()}
                            }
                        }
                    }
                }
            } else {
            // ===== MAIN CONTENT =====
            main { class: "main-content flex-1 flex flex-col overflow-hidden py-5 px-6",

                div { class: "grid grid-cols-2 gap-5 max-[860px]:grid-cols-1",

                    // ---- Create Project ----
                    section { class: "card",
                        h2 { class: "m-0 text-[15px] font-bold text-bkl-text tracking-[0.02em]",
                            {language.read().text("Create Project").into_owned()}
                        }
                        p { class: "mt-0.5 text-xs text-bkl-text-muted",
                            {language.read().text("Generate a template-based project and open it in VS Code.").into_owned()}
                        }

                        div { class: "mt-4",
                            div { class: "flex flex-wrap items-center gap-2",
                                label { class: "text-[13px] font-semibold text-bkl-text-muted",
                                    {language.read().text("Project name").into_owned()}
                                }
                                input {
                                    class: "input",
                                    value: "{project_name}",
                                    oninput: move |ev| project_name.set(ev.value()),
                                }
                                button {
                                    class: "btn-primary",
                                    onclick: move |_| {
                                        let ref_value =
                                            template_ref_value.read().trim().to_string();
                                        let selected_ref =
                                            (!ref_value.is_empty()).then_some(ref_value);
                                        let (branch, tag) = if *template_ref_is_tag.read() {
                                            (None, selected_ref)
                                        } else {
                                            (selected_ref, None)
                                        };
                                        let name = project_name.read().clone();
                                        let open_after = *vscode_open_enabled.read();
                                        spawn(async move {
                                            if let Some(path) =
                                                rfd::AsyncFileDialog::new().set_title(language.read().text("Choose project folder").into_owned()).pick_folder().await
                                            {
                                                actions.send(AppAction::CreateProject {
                                                    name,
                                                    parent_path: path
                                                        .path()
                                                        .to_string_lossy()
                                                        .to_string(),
                                                    branch,
                                                    tag,
                                                    open_after,
                                                });
                                            }
                                        });
                                    },
                                    {language.read().text("Create").into_owned()}
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
                                    {language.read().text("Specify template version").into_owned()}
                                }
                            }

                            if *show_version_opts.read() {
                                div { class: "flex flex-wrap items-center gap-2",
                                    label { class: "text-[13px] font-semibold text-bkl-text-muted",
                                        {language.read().text("Version").into_owned()}
                                    }
                                    div { class: "flex gap-1",
                                        button {
                                            class: if !*template_ref_is_tag.read() { "btn-chip btn-chip-active" } else { "btn-chip" },
                                            onclick: move |_| template_ref_is_tag.set(false),
                                            {language.read().text("Branch").into_owned()}
                                        }
                                        button {
                                            class: if *template_ref_is_tag.read() { "btn-chip btn-chip-active" } else { "btn-chip" },
                                            onclick: move |_| template_ref_is_tag.set(true),
                                            {language.read().text("Tag").into_owned()}
                                        }
                                    }
                                    input {
                                        class: "input",
                                        placeholder: if *template_ref_is_tag.read() { language.read().text("e.g. v1.0.0 (blank = latest)").into_owned() } else { language.read().text("e.g. main (blank = HEAD)").into_owned() },
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
                                    {language.read().text("Open VS Code after creation").into_owned()}
                                }
                            }

                            a {
                                class: "template-link",
                                href: "{parameter::TEMPLATE_URL}",
                                {language.read().text("View template repository").into_owned()}
                            }
                        }
                    }

                    // ---- DAP Server ----
                    section { class: "card",
                        div { class: "flex items-center justify-between",
                            div {
                                h2 { class: "m-0 text-[15px] font-bold text-bkl-text tracking-[0.02em]",
                                    {language.read().text("probe-rs DAP Server").into_owned()}
                                }
                                p { class: "mt-0.5 text-xs text-bkl-text-muted",
                                    {language.read().text("Launch a local DAP server for debugging.").into_owned()}
                                }
                            }
                            div { class: "flex items-center gap-1.5 text-[11px] text-bkl-text-muted",
                                span { class: "size-2 rounded-full inline-block shrink-0 {helpers::dap_dot_class(*dap_running.read())}" }
                                span {
                                    if *dap_running.read() {
                                        {language.read().text("Running").into_owned()}
                                    } else {
                                        {language.read().text("Stopped").into_owned()}
                                    }
                                }
                            }
                        }

                        div { class: "mt-4",
                            div { class: "mb-3.5 pb-3.5 border-b border-bkl-border",
                                div { class: "flex items-center justify-between",
                                    span { class: "text-[13px] font-semibold text-bkl-text-muted",
                                        {language.read().text("Connected Device").into_owned()}
                                    }
                                    button {
                                        class: "btn-chip",
                                        disabled: *detecting.read(),
                                        onclick: move |_| actions.send(AppAction::DetectDevice),
                                        if *detecting.read() {
                                            {language.read().text("Detecting...").into_owned()}
                                        } else {
                                            {language.read().text("Detect").into_owned()}
                                        }
                                    }
                                }
                                if !probe_name.read().is_empty() {
                                    div { class: "probe-info-grid",
                                        div { class: "flex items-baseline gap-2 min-w-0",
                                            span { class: "text-[10px] font-bold text-bkl-text-faint uppercase tracking-[0.04em] shrink-0",
                                                {language.read().text("Probe").into_owned()}
                                            }
                                            span { class: "text-xs text-bkl-text-muted font-mono overflow-hidden text-ellipsis whitespace-nowrap",
                                                {language.read().text(&probe_name.read()).into_owned()}
                                            }
                                        }
                                        if !chip_name.read().is_empty() {
                                            div { class: "flex items-baseline gap-2 min-w-0",
                                                span { class: "text-[10px] font-bold text-bkl-text-faint uppercase tracking-[0.04em] shrink-0",
                                                    {language.read().text("Chip").into_owned()}
                                                }
                                                span { class: "text-xs font-mono font-semibold text-bkl-orange-light overflow-hidden text-ellipsis whitespace-nowrap",
                                                    {language.read().text(&chip_name.read()).into_owned()}
                                                }
                                            }
                                        }
                                        if !chip_cores.read().is_empty() {
                                            div { class: "flex items-baseline gap-2 min-w-0",
                                                span { class: "text-[10px] font-bold text-bkl-text-faint uppercase tracking-[0.04em] shrink-0",
                                                    {language.read().text("Core").into_owned()}
                                                }
                                                span { class: "text-xs text-bkl-text-muted font-mono overflow-hidden text-ellipsis whitespace-nowrap",
                                                    "{chip_cores}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            div { class: "flex items-center justify-between gap-3 flex-wrap",
                                div { class: "flex items-center gap-2",
                                    button {
                                        class: "btn-primary",
                                        disabled: *dap_running.read() || *dap_stopping.read(),
                                        onclick: move |_| actions.send(AppAction::StartDap(settings::ServerConfig {
                                            bind_ip: dap_ip.read().clone(),
                                            connect_host: server_config.connect_host.clone(),
                                            dap_port: dap_port.read().clone(),
                                            upload_port: upload_port.read().clone(),
                                        })),
                                        {language.read().text("Run").into_owned()}
                                    }
                                    button {
                                        class: "btn-danger",
                                        disabled: !*dap_running.read() || *dap_stopping.read(),
                                        onclick: move |_| actions.send(AppAction::StopDap),
                                        if *dap_stopping.read() { {language.read().text("Stopping...").into_owned()} } else { {language.read().text("Stop").into_owned()} }
                                    }
                                }
                                button {
                                    class: "btn-chip",
                                    onclick: move |_| {
                                        let current = *show_dap_advanced.peek();
                                        show_dap_advanced.set(!current);
                                        if !current {
                                            actions.send(AppAction::Cache(CacheAction::Refresh));
                                        }
                                    },
                                    if *show_dap_advanced.read() {
                                        {language.read().text("Hide Advanced Settings").into_owned()}
                                    } else {
                                        {language.read().text("Advanced Settings").into_owned()}
                                    }
                                }
                            }

                            if *show_dap_advanced.read() {
                                div { class: "mt-3.5 p-3 rounded-[10px] bg-bkl-surface-inset border border-bkl-border flex flex-col gap-2.5",
                                    div { class: "grid grid-cols-3 gap-2.5 max-[520px]:grid-cols-1",
                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[11px] font-semibold text-bkl-text-muted",
                                                {language.read().text("Bind IP").into_owned()}
                                            }
                                            input {
                                                class: "input min-w-0 w-full text-xs",
                                                value: "{dap_ip}",
                                                disabled: *dap_running.read() || *dap_stopping.read(),
                                                oninput: move |event| dap_ip.set(event.value()),
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[11px] font-semibold text-bkl-text-muted",
                                                {language.read().text("DAP Port").into_owned()}
                                            }
                                            input {
                                                class: "input min-w-0 w-full text-xs",
                                                value: "{dap_port}",
                                                disabled: *dap_running.read() || *dap_stopping.read(),
                                                oninput: move |event| dap_port.set(event.value()),
                                            }
                                        }
                                        div { class: "flex flex-col gap-1",
                                            label { class: "text-[11px] font-semibold text-bkl-text-muted",
                                                {language.read().text("Upload Port").into_owned()}
                                            }
                                            input {
                                                class: "input min-w-0 w-full text-xs",
                                                value: "{upload_port}",
                                                disabled: *dap_running.read() || *dap_stopping.read(),
                                                oninput: move |event| upload_port.set(event.value()),
                                            }
                                        }
                                    }
                                    p { class: "m-0 text-[11px] text-bkl-text-faint",
                                        {language.read().text("Dev Container connects through host.docker.internal. Changes apply on Run.").into_owned()}
                                    }
                                    h3 { class: "m-0 text-xs font-semibold text-bkl-text-muted",
                                        {language.read().text("ELF Cache").into_owned()}
                                    }
                                    p { class: "m-0 text-[11px] text-bkl-text-faint",
                                        {language.read().text("Oldest unused artifacts are removed when capacity is exceeded. No expiry. Active debug artifacts are protected from deletion.").into_owned()}
                                    }
                                    if let Some(usage) = cache_usage.read().as_ref() {
                                        p { class: "m-0 text-xs text-bkl-text-muted",
                                            {language.read().text(&format!("Cache: {} MiB / {} MiB; {} files; {} active",
                                                usage.bytes.div_ceil(1024 * 1024),
                                                usage.limit / (1024 * 1024), usage.files, usage.active_files)).into_owned()}
                                        }
                                    }
                                    div { class: "flex items-center gap-2 flex-wrap",
                                        label { class: "text-[11px] font-semibold text-bkl-text-muted",
                                            {language.read().text("Capacity (MiB)").into_owned()}
                                        }
                                        input {
                                            class: "input min-w-0 text-xs",
                                            r#type: "number",
                                            min: "128",
                                            max: "1048576",
                                            value: "{cache_limit_mib}",
                                            disabled: *cache_busy.read(),
                                            oninput: move |event| cache_limit_mib.set(event.value()),
                                        }
                                        button {
                                            class: "btn-chip",
                                            disabled: *cache_busy.read(),
                                            onclick: move |_| actions.send(AppAction::Cache(CacheAction::Save(cache_limit_mib.read().clone()))),
                                            {language.read().text("Save capacity").into_owned()}
                                        }
                                        button {
                                            class: "btn-chip",
                                            disabled: *cache_busy.read(),
                                            onclick: move |_| actions.send(AppAction::Cache(CacheAction::Refresh)),
                                            {language.read().text("Refresh usage").into_owned()}
                                        }
                                        button {
                                            class: "btn-danger",
                                            disabled: *cache_busy.read(),
                                            onclick: move |_| actions.send(AppAction::Cache(CacheAction::ClearUnused)),
                                            {language.read().text("Delete unused cache").into_owned()}
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // ---- Log ----
                section { class: "card mt-5 flex-1 flex flex-col min-h-0",
                    div { class: "flex items-center justify-between",
                        div {
                            h2 { class: "m-0 text-[15px] font-bold text-bkl-text tracking-[0.02em]",
                                {language.read().text("Log").into_owned()}
                            }
                            p { class: "mt-0.5 text-xs text-bkl-text-muted",
                                {language.read().text("Build and runtime output.").into_owned()}
                            }
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "btn-chip",
                                disabled: logs.read().is_empty(),
                                onclick: move |_| {
                                    logs.set(Vec::new());
                                    if let Ok(mut buffer) = crate::display_buffer().lock() {
                                        buffer.clear();
                                    }
                                },
                                {language.read().text("Clear").into_owned()}
                            }
                            button {
                                class: "btn-chip",
                                disabled: logs.read().is_empty(),
                                onclick: move |_| {
                                    let text = logs.read().iter().map(|line| localized_log_line(line, *language.read())).collect::<Vec<_>>().join("\n");
                                    if let Err(error) = arboard::Clipboard::new().and_then(|mut clipboard| clipboard.set_text(text)) {
                                        let message = format!("Failed to copy logs: {error}");
                                        crate::log_error(message.clone());
                                        last_error.set(Some(message));
                                    }
                                },
                                {language.read().text("Copy").into_owned()}
                            }
                        }
                    }
                    div { class: "log-viewer",
                        for (idx , line) in logs.read().iter().enumerate() {
                            {
                                let display = logger::present_log_line(line);
                                rsx! {
                                    div {
                                        key: "{idx}",
                                        class: "log-line {display.class}",
                                        span { class: "text-bkl-text-faint shrink-0 select-all",
                                            "{display.timestamp}"
                                        }
                                        span { class: "log-badge {display.badge_class}",
                                            {language.read().text(display.level).into_owned()}
                                        }
                                        span { class: "break-all", {language.read().text(display.message).into_owned()} }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            }

            // ---- Notifications ----
            if let Some(message) = last_error.read().clone().or_else(|| doctor_notice.read().clone()) {
                div {
                    class: "error-toast",
                    role: "alert",
                    style: "z-index: 150;",
                    div { class: "flex items-start justify-between gap-3",
                        span {
                            {format!("[{}] {}", language.read().text(if last_error.read().is_some() { "ERROR" } else { "Doctor" }), language.read().text(&message))}
                        }
                        button {
                            class: "btn-chip",
                            style: "flex-shrink: 0; min-width: 32px; min-height: 32px; font-size: 20px;",
                            title: language.read().text("Dismiss notification").into_owned(),
                            aria_label: language.read().text("Dismiss notification").into_owned(),
                            onclick: move |_| {
                                if last_error.peek().is_some() {
                                    last_error.set(None);
                                } else {
                                    doctor_notice.set(None);
                                }
                            },
                            "\u{00d7}"
                        }
                    }
                    if doctor_notice.read().is_some() && last_error.read().is_none() {
                        button {
                            class: "btn-chip",
                            onclick: move |_| actions.send(AppAction::OpenHelp(Some("environment-setup"))),
                            {language.read().text("Open Help").into_owned()}
                        }
                    }
                }
            }

            // ---- Docker startup prompt ----
            if docker_status.read().contains("Off") && !*docker_prompt_dismissed.read()
                && !*show_splash.read() && !*show_doctor.read() && !*startup_diagnosis_pending.read()
            {
                div { class: "modal-overlay",
                    div { class: "modal",
                        div { class: "flex justify-end",
                            button {
                                class: "btn-chip",
                                title: language.read().text("Dismiss Docker startup prompt").into_owned(),
                                aria_label: language.read().text("Dismiss Docker startup prompt").into_owned(),
                                onclick: move |_| docker_prompt_dismissed.set(true),
                                "\u{00d7}"
                            }
                        }
                        h3 { class: "m-0 mb-3 text-base font-bold text-bkl-text", "Rancher Desktop" }
                        p { class: "m-0 mb-1 text-[13px] text-bkl-text-muted",
                            {language.read().text("Docker is not running.").into_owned()}
                        }
                        p { class: "m-0 mb-1 text-[13px] text-bkl-text-muted",
                            {language.read().text("Start Rancher Desktop now?").into_owned()}
                        }
                        div { class: "flex gap-2 mt-5",
                            button {
                                class: "btn-primary",
                                onclick: move |_| {
                                    actions.send(AppAction::StartDocker);
                                    docker_prompt_dismissed.set(true);
                                },
                                {language.read().text("Start").into_owned()}
                            }
                            button {
                                class: "btn-chip",
                                onclick: move |_| docker_prompt_dismissed.set(true),
                                {language.read().text("Not now").into_owned()}
                            }
                        }
                    }
                }
            }

            // ---- Reset confirmation modal ----
            if *show_reset_confirm.read() {
                div { class: "modal-overlay",
                    div { class: "modal",
                        div { class: "flex justify-end",
                            button {
                                class: "btn-chip",
                                title: language.read().text("Cancel reset").into_owned(),
                                aria_label: language.read().text("Cancel reset").into_owned(),
                                onclick: move |_| show_reset_confirm.set(false),
                                "\u{00d7}"
                            }
                        }
                        h3 { class: "m-0 mb-3 text-base font-bold text-bkl-text", {language.read().text("Reset Settings").into_owned()} }
                        p { class: "m-0 mb-1 text-[13px] text-bkl-text-muted",
                            {language.read().text("All settings including project history will be cleared.").into_owned()}
                        }
                        p { class: "m-0 mb-1 text-[13px] text-bkl-text-muted",
                            {language.read().text("The application will restart from the splash screen.").into_owned()}
                        }
                        div { class: "flex gap-2 mt-5",
                            button {
                                class: "btn-danger",
                                onclick: move |_| {
                                    match settings::reset_all() {
                                        Ok(()) => {
                                            language.set(Language::English);
                                            history.set(vec![]);
                                            show_reset_confirm.set(false);
                                            show_splash.set(true);
                                            actions.send(AppAction::Cache(CacheAction::Refresh));
                                        }
                                        Err(error) => {
                                            crate::log_error(format!("Failed to reset settings: {error}"));
                                            last_error.set(Some(error.to_string()));
                                        }
                                    }
                                },
                                {language.read().text("Reset").into_owned()}
                            }
                            button {
                                class: "btn-chip",
                                onclick: move |_| show_reset_confirm.set(false),
                                {language.read().text("Cancel").into_owned()}
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
