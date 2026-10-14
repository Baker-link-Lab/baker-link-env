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
            let formatted = if msg.len() >= 23 && is_valid_log_timestamp(&msg[..23]) {
                msg
            } else {
                let (level, clean_msg) = if let Some(stripped) = msg.strip_prefix("[ERROR] ") {
                    (log::Level::Error, stripped)
                } else if let Some(stripped) = msg.strip_prefix("[INFO] ") {
                    (log::Level::Info, stripped)
                } else if msg.starts_with("[ERROR]") {
                    (
                        log::Level::Error,
                        msg.trim_start_matches("[ERROR]").trim_start(),
                    )
                } else if msg.starts_with("[INFO]") {
                    (
                        log::Level::Info,
                        msg.trim_start_matches("[INFO]").trim_start(),
                    )
                } else {
                    (log::Level::Info, msg.as_str())
                };
                format!(
                    "{}[{}]: {}",
                    Self::get_timestamp(),
                    level.as_str(),
                    clean_msg
                )
            };
            self.buffer.push(formatted);
        }
    }

    pub fn clear(&mut self) {
        self.buffer.clear();
        while self.rx.try_recv().is_ok() {}
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
    let has_timestamp = line.len() >= 23 && is_valid_log_timestamp(&line[..23]);
    let (timestamp, message) = if has_timestamp {
        let ts = &line[..23];
        let rest = &line[23..];
        let msg = if let Some(position) = rest.find("]: ") {
            &rest[position + 3..]
        } else {
            rest.trim_start()
        };
        (ts, msg)
    } else {
        let msg = if let Some(position) = line.find("]: ") {
            &line[position + 3..]
        } else {
            line
        };
        ("", msg)
    };

    LogLine {
        class,
        badge_class,
        level,
        timestamp,
        message,
    }
}

fn is_valid_log_timestamp(s: &str) -> bool {
    if s.len() != 23 {
        return false;
    }
    let b = s.as_bytes();
    b[0..4].iter().all(u8::is_ascii_digit)
        && b[4] == b'-'
        && b[5..7].iter().all(u8::is_ascii_digit)
        && b[7] == b'-'
        && b[8..10].iter().all(u8::is_ascii_digit)
        && b[10] == b' '
        && b[11..13].iter().all(u8::is_ascii_digit)
        && b[13] == b':'
        && b[14..16].iter().all(u8::is_ascii_digit)
        && b[16] == b':'
        && b[17..19].iter().all(u8::is_ascii_digit)
        && b[19] == b'.'
        && b[20..23].iter().all(u8::is_ascii_digit)
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

    #[test]
    fn handles_unformatted_log_line_without_splitting_text() {
        let display = present_log_line("ELF upload server listening on 0.0.0.0:50002");

        assert_eq!(display.timestamp, "");
        assert_eq!(
            display.message,
            "ELF upload server listening on 0.0.0.0:50002"
        );
    }
}
