use std::{io::Write, time::Instant};
use chrono::{DateTime, Local};

pub fn read_line() -> String {
    let stdin = std::io::stdin();
    let mut buffer = String::new();
    std::io::stdout().flush().expect("Failed to flush stdout");
    buffer.clear();
    stdin.read_line(&mut buffer).expect("An error has occoured while reading line");
    buffer.trim().to_string()
}

pub fn format_time(time: DateTime<Local>) -> String {
    time.format("%H:%M:%S").to_string()
}

pub fn format_u32_time(time: u32) -> String {
    format!("{:02}:{:02}", time / 3600, (time / 60) % 60)
}

pub fn measure<F, R>(func: F) -> (R, String)
where
    F: FnOnce() -> R,
{
    let start = Instant::now();
    let result = func();
    let duration = start.elapsed().as_millis();
    let elapsed = format!("{} ms", duration);

    (result, elapsed)
}