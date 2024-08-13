pub struct DisplayBuffer {
    pub buffer: Vec<String>,
    log_level: log::Level,
    pub tx: std::sync::mpsc::Sender<String>,
    rx: std::sync::mpsc::Receiver<String>,
}

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
        now.format("%Y-%m-%d %H:%M:%S%.3f").to_string()
    }

    pub fn log_info(&mut self, msg: String) {
        if self.log_level >= log::Level::Info {
            let _ = self.tx.send(format!("{}[INFO]: {}", Self::get_timestamp(), msg));
        }
    }

    pub fn log_debug(&mut self, msg: String) {
        if self.log_level >= log::Level::Debug {
            let _ = self.tx.send(format!("{}[DEBUG]: {}", Self::get_timestamp(), msg));
        }
    }

    pub fn log_error(&mut self, msg: String) {
        if self.log_level >= log::Level::Error {
            let _ = self.tx.send(format!("{}[ERROR]: {}", Self::get_timestamp(), msg));
        }
    }
}
