use std::sync::mpsc;

const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S%.3f";

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
