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

pub struct LogLine<'a> {
    pub class: &'static str,
    pub badge_class: &'static str,
    pub level: &'static str,
    pub timestamp: &'a str,
    pub message: &'a str,
}

pub fn present_log_line(line: &str) -> LogLine<'_> {
    let (class, badge_class, level) = if line.contains("[ERROR]") {
        ("log-error", "log-badge-error", "ERR")
    } else if line.contains("[INFO]") {
        ("log-info", "log-badge-info", "INF")
    } else {
        ("log-debug", "log-badge-default", "LOG")
    };
    let timestamp = line.get(..23).unwrap_or("");
    let message = if let Some(position) = line.find("]: ") {
        &line[position + 3..]
    } else if line.len() > 23 {
        line.get(23..).unwrap_or(line)
    } else {
        line
    };

    LogLine {
        class,
        badge_class,
        level,
        timestamp,
        message,
    }
}

#[cfg(test)]
mod tests {
    use super::present_log_line;

    #[test]
    fn parses_log_line_for_display() {
        let display = present_log_line("2026-10-04 12:34:56.789[ERROR]: upload failed");

        assert_eq!(display.class, "log-error");
        assert_eq!(display.badge_class, "log-badge-error");
        assert_eq!(display.level, "ERR");
        assert_eq!(display.timestamp, "2026-10-04 12:34:56.789");
        assert_eq!(display.message, "upload failed");
    }

    #[test]
    fn handles_short_and_non_ascii_log_lines() {
        let display = present_log_line("短い");

        assert_eq!(display.timestamp, "");
        assert_eq!(display.message, "短い");
    }
}
