#![forbid(unsafe_code)]

use std::env;

fn main() {
    match env::args().nth(1).as_deref() {
        Some("version" | "--version" | "-V") => {
            println!("{}", sure_core::version_string());
        }
        _ => {
            println!("SURE — Software Understanding & Reality Evaluation");
            println!("AI says it's done. Be SURE.");
            println!("Bootstrap repository: read MASTER_PROMPT.md.");
        }
    }
}
