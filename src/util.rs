use std::io::Write;

pub fn read_line() -> String {
    let stdin = std::io::stdin();
    let mut buffer = String::new();
    std::io::stdout().flush().expect("Failed to flush stdout");
    buffer.clear();
    stdin.read_line(&mut buffer).expect("An error has occoured while reading line");
    buffer.trim().to_string()
}