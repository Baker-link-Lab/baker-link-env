use std::sync::{mpsc, Mutex, OnceLock};

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

// ---- Global singleton ----

static DISPLAY_BUFFER: OnceLock<Mutex<DisplayBuffer>> = OnceLock::new();

pub fn display_buffer() -> &'static Mutex<DisplayBuffer> {
    DISPLAY_BUFFER.get_or_init(|| Mutex::new(DisplayBuffer::new()))
}

pub fn log_info(msg: impl Into<String>) {
    if let Ok(mut buf) = display_buffer().lock() {
        buf.log_info(msg.into());
    }
}

pub fn log_error(msg: impl Into<String>) {
    if let Ok(mut buf) = display_buffer().lock() {
        buf.log_error(msg.into());
    }
}

// ---- DisplayBuffer ----

pub struct DisplayBuffer {
    pub buffer: Vec<String>,
    tx: mpsc::Sender<String>,
    rx: mpsc::Receiver<String>,
}

impl DisplayBuffer {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            buffer: Vec::new(),
            tx,
            rx,
        }
    }

    pub fn sender(&self) -> mpsc::Sender<String> {
        self.tx.clone()
    }

    pub fn channel_recv(&mut self) {
        for msg in self.rx.try_iter() {
            self.buffer.push(msg);
        }
    }

    pub fn log_info(&mut self, msg: String) {
        self.log_with_level(log::Level::Info, msg);
    }

    pub fn log_error(&mut self, msg: String) {
        self.log_with_level(log::Level::Error, msg);
    }

    fn log_with_level(&mut self, level: log::Level, msg: String) {
        let _ = self.tx.send(format!(
            "{}[{}]: {}",
            Self::get_timestamp(),
            level.as_str(),
            msg
        ));
    }

    fn get_timestamp() -> String {
        chrono::Local::now().format(TIMESTAMP_FORMAT).to_string()
    }
}

impl Default for DisplayBuffer {
    fn default() -> Self {
        Self::new()
    }
}

// ---- Log line display helpers ----

pub fn log_level_class(line: &str) -> &'static str {
    if line.contains("[ERROR]") {
        "log-error"
    } else if line.contains("[INFO]") {
        "log-info"
    } else {
        "log-debug"
    }
}

pub fn log_badge_class(line: &str) -> &'static str {
    if line.contains("[ERROR]") {
        "log-badge-error"
    } else if line.contains("[INFO]") {
        "log-badge-info"
    } else {
        "log-badge-default"
    }
}

pub fn extract_level(line: &str) -> &'static str {
    if line.contains("[ERROR]") {
        "ERR"
    } else if line.contains("[INFO]") {
        "INF"
    } else {
        "LOG"
    }
}

pub fn extract_timestamp(line: &str) -> &str {
    if line.len() >= 23 {
        &line[..23]
    } else {
        ""
    }
}

pub fn extract_message(line: &str) -> &str {
    if let Some(pos) = line.find("]: ") {
        &line[pos + 3..]
    } else if line.len() > 23 {
        &line[23..]
    } else {
        line
    }
}
