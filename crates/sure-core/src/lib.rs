#![forbid(unsafe_code)]

pub const PRODUCT_NAME: &str = "SURE";
pub const PRODUCT_EXPANSION: &str = "Software Understanding & Reality Evaluation";

#[must_use]
pub fn version_string() -> String {
    format!("{PRODUCT_NAME} {}", env!("CARGO_PKG_VERSION"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_is_not_empty() {
        assert!(!version_string().is_empty());
    }
}
