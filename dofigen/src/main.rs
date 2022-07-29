use dofigen_lib::{generate, parse_from_json};
use serde_json::Value;
use std::io::{self, BufRead};

fn main() -> io::Result<()> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        buffer.push_str(&line.unwrap());
        let p: Result<Value, _> = serde_json::from_str(&buffer);
        match p {
            Ok(_json_val) => break,
            Err(_) => buffer.push_str("\n"),
        };
    }

    let dockerfile_content = generate(parse_from_json(buffer));
    print!("{}", dockerfile_content);
    Ok(())
}
