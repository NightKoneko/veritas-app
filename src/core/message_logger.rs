#[derive(Default)]
pub struct MessageLogger {
    logs: Vec<String>
}

impl MessageLogger {
    pub fn log(&mut self, message: &str) {
        let timestamp = chrono::Local::now().format("%H:%M:%S");
        let formatted = format!("[{}] {}", timestamp, message);
        self.logs.push(formatted);
        
        if self.logs.len() > 1000 {
            self.logs.remove(0);
        }
    }
    pub fn get_text(&self) -> String {
        self.logs.join("\n")
    }
}