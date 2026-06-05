use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::OnceLock,
    time::{SystemTime, UNIX_EPOCH},
};

static LOG_FILE_PATH: OnceLock<PathBuf> = OnceLock::new();

pub fn init(log_directory: PathBuf) {
    let _ = fs::create_dir_all(&log_directory);
    let _ = LOG_FILE_PATH.set(log_directory.join("waey.log"));
    install_panic_hook();
    info("logger initialized");
}

pub fn info(message: impl AsRef<str>) {
    write_line("INFO", message.as_ref());
}

pub fn warn(message: impl AsRef<str>) {
    write_line("WARN", message.as_ref());
}

pub fn error(message: impl AsRef<str>) {
    write_line("ERROR", message.as_ref());
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|panic_info| {
        error(format!("panic: {panic_info}"));
    }));
}

fn write_line(level: &str, message: &str) {
    let path = LOG_FILE_PATH
        .get()
        .cloned()
        .unwrap_or_else(|| std::env::temp_dir().join("waey").join("waey.log"));

    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(file, "{} [{level}] {message}", timestamp());
    }
}

fn timestamp() -> String {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => format!("{}.{:03}", duration.as_secs(), duration.subsec_millis()),
        Err(_) => "0.000".to_string(),
    }
}
