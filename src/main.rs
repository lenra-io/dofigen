extern crate serde;
extern crate schemafy_core;
extern crate serde_json;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::io::{self, BufRead};

schemafy::schemafy!(
    root: Image
    "schema/image.json"
);

schemafy::schemafy!(
    root: Stage
    "schema/stage.json"
);

// #[derive(Serialize, Deserialize)]
// struct Stage {
//     image: String,
//     #[serde(rename = "rootScript")]
//     root_script: Option<Vec<String>>,
//     script: Vec<String>,
//     workdir: Option<String>
// }

fn main() -> io::Result<()> {
    let mut buffer = String::new();
    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        buffer.push_str(&line.unwrap());
        let p: Result<Value, _> = serde_json::from_str(&buffer);
        match p {
            Ok(_json_val) => {
                println!("{}", buffer);
                handle_json(&serde_json::from_str(&buffer).unwrap());
                break;
            }
            Err(_) => {}
        };
        buffer.push_str("\n");
    }
    Ok(())
}

fn handle_json(stage: &Stage) {
    print!("{}", serde_json::to_string(stage).unwrap());
}
