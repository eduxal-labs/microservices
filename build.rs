fn main() {
    if let Ok(content) = std::fs::read_to_string(".env") {
        for line in content.lines() {
            // Parse KEY=VALUE pairs
            if let Some((key, value)) = line.split_once('=') {
                // Set as cargo environment variable for compile-time access
                println!("cargo:rustc-env={}={}", key.trim(), value.trim());
            }
        }
    }
}
