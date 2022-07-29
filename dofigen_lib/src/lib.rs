pub fn generate(input: String) -> String {
    input
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_basic_json() {
        let json = r#"
        {
            "builders": [
                {
                    "name": "builder",
                    "image": "ekidd/rust-musl-builder",
                    "add": [
                        "*"
                    ],
                    "script": [
                        "cargo build --release"
                    ]
                }
            ],
            "image": "scratch",
            "artifacts": [
                {
                    "builder": "builder",
                    "source": "/home/rust/src/target/x86_64-unknown-linux-musl/release/template-rust",
                    "destination": "/app"
                }
            ]
        }"#;
        let expected_dockerfile = r#"
        {
            "name": "John Doe",
            "age": 43,
            "phones": [
                "+44 1234567",
                "+44 2345678"
            ]
        }"#;
        let dockerfile = generate(json.to_string());
        assert_eq!(dockerfile, expected_dockerfile.to_string());
    }
}
