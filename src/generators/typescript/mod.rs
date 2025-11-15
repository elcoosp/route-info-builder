pub mod client;
pub mod hooks;
pub use client::*;
pub use hooks::*;
pub fn format_ts_code(code: &str) -> Result<String, Box<dyn std::error::Error>> {
    // For now, we'll use a simple formatter since deno_ast might be heavy
    Ok(code.to_string())
}
