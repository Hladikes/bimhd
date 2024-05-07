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

pub fn measure<F, R>(func: F) -> (R, u128)
where
    F: FnOnce() -> R,
{
    // Start the timer
    let start = Instant::now();

    // Execute the function
    let result = func();

    // Calculate the duration
    let duration = start.elapsed().as_micros();

    (result, duration) // Return the result and the duration
}