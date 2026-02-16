pub struct DisplayBuffer {
    pub buffer: Vec<String>,
    log_level: log::Level,
    pub tx: std::sync::mpsc::Sender<String>,
    rx: std::sync::mpsc::Receiver<String>,
}

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

impl DisplayBuffer {
    pub fn new() -> Self {
        let (tx, rx) = std::sync::mpsc::channel();
        Self {
            buffer: Vec::new(),
            log_level: log::Level::Info,
            tx,
            rx,
        }
    }

    pub fn channel_recv(&mut self) {
        for msg in self.rx.try_iter() {
            self.buffer.push(msg);
        }
    }

    fn get_timestamp() -> String {
        let now = chrono::Local::now();
        now.format(TIMESTAMP_FORMAT).to_string()
    }

    pub fn log_info(&mut self, msg: String) {
        self.log_with_level(log::Level::Info, "INFO", msg);
    }

    pub fn log_error(&mut self, msg: String) {
        self.log_with_level(log::Level::Error, "ERROR", msg);
    }

    fn log_with_level(&mut self, level: log::Level, label: &str, msg: String) {
        if self.log_level >= level {
            let _ = self
                .tx
                .send(format!("{}[{}]: {}", Self::get_timestamp(), label, msg));
        }
    }
}
