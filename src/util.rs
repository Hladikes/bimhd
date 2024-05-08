use std::time::Instant;

pub fn format_seconds_to_minutes(seconds: u32) -> String {
    let minutes = seconds / 60;
    let seconds = seconds % 60;
    format!("{:02}:{:02}m", minutes, seconds)
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