use dofigen_lib::{generate, from_json_reader, structs::Image};
use std::io::Result;

fn main() -> Result<()> {
    let image: Image = from_json_reader(std::io::stdin().lock());
    let dockerfile_content = generate(image);
    print!("{}", dockerfile_content);
    Ok(())
}
