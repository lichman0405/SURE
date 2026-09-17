//! Production-path/context filter for candidate scanner hits.
//!
//! `P6-T002`'s acceptance: Test/example/docs mocks are distinguished from likely
//! real user paths where possible.

use std::path::Path;

use crate::components::ComponentGraph;

/// Where a candidate pattern was found.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CandidateContext {
    /// Found in a test file or test directory.
    Test,
    /// Found in an example file or example directory.
    Example,
    /// Found in documentation.
    Doc,
    /// Found in a mock, stub, or fixture file/directory.
    MockFixture,
    /// Found in likely production or product code.
    Product,
}

/// Classify a source file path by its likely role in the project.
///
/// Heuristics are conservative: a path is flagged as test/example/doc/mock only
/// when it is clearly in a conventional directory or has a conventional file
/// name. When uncertain, [`CandidateContext::Product`] is returned.
///
/// Works on both Unix (`/`) and Windows (`\`) path separators.
///
/// `root` and `graph` are available for future component-aware heuristics;
/// the current implementation uses conventional directory and file-name patterns.
#[must_use]
pub fn classify_path(
    path: &Path,
    _root: &Path,
    _graph: Option<&ComponentGraph>,
) -> CandidateContext {
    // Check file name segments first (more specific than directory).
    if let Some(file_name) = path.file_name().and_then(|s| s.to_str()) {
        for segment in file_name.split('.') {
            let lower = segment.to_lowercase();
            if lower == "test" || lower == "spec" {
                return CandidateContext::Test;
            }
            if lower == "mock" || lower == "stub" {
                return CandidateContext::MockFixture;
            }
        }
    }

    // Check each path component against conventional directory names.
    for component in path.components() {
        if let Some(name) = component.as_os_str().to_str() {
            let lower = name.to_lowercase();
            match lower.as_str() {
                "tests" | "test" | "__tests__" => return CandidateContext::Test,
                "examples" | "example" => return CandidateContext::Example,
                "docs" | "doc" => return CandidateContext::Doc,
                "fixtures" | "mocks" | "stubs" => return CandidateContext::MockFixture,
                _ => {}
            }
        }
    }

    CandidateContext::Product
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    fn classify(path: &str) -> CandidateContext {
        classify_path(Path::new(path), Path::new(""), None)
    }

    #[test]
    fn tests_directories_are_test_context() {
        assert_eq!(classify("tests/foo.rs"), CandidateContext::Test);
        assert_eq!(classify("test/bar.js"), CandidateContext::Test);
        assert_eq!(classify("__tests__/baz.ts"), CandidateContext::Test);
    }

    #[test]
    fn examples_directories_are_example_context() {
        assert_eq!(classify("examples/demo.rs"), CandidateContext::Example);
        assert_eq!(classify("example/usage.js"), CandidateContext::Example);
    }

    #[test]
    fn docs_directories_are_doc_context() {
        assert_eq!(classify("docs/readme.md"), CandidateContext::Doc);
        assert_eq!(classify("doc/api.rst"), CandidateContext::Doc);
    }

    #[test]
    fn fixture_directories_are_mock_fixture_context() {
        assert_eq!(classify("fixtures/data.rs"), CandidateContext::MockFixture);
        assert_eq!(classify("mocks/server.js"), CandidateContext::MockFixture);
        assert_eq!(classify("stubs/impl.ts"), CandidateContext::MockFixture);
    }

    #[test]
    fn test_and_spec_file_names_are_test_context() {
        assert_eq!(classify("src/foo.test.js"), CandidateContext::Test);
        assert_eq!(classify("lib/bar.spec.rs"), CandidateContext::Test);
        assert_eq!(classify("app/baz.test.ts"), CandidateContext::Test);
    }

    #[test]
    fn mock_and_stub_file_names_are_mock_fixture_context() {
        assert_eq!(classify("src/foo.mock.js"), CandidateContext::MockFixture);
        assert_eq!(classify("lib/bar.stub.rs"), CandidateContext::MockFixture);
    }

    #[test]
    fn ordinary_src_or_app_is_product_context() {
        assert_eq!(classify("src/main.rs"), CandidateContext::Product);
        assert_eq!(classify("app/index.js"), CandidateContext::Product);
        assert_eq!(classify("lib/lib.rs"), CandidateContext::Product);
    }

    #[test]
    fn test_helper_in_src_remains_product() {
        assert_eq!(classify("src/test_helper.rs"), CandidateContext::Product);
        assert_eq!(classify("app/testing_utils.js"), CandidateContext::Product);
    }

    #[test]
    fn windows_separators_work() {
        assert_eq!(classify("tests\\foo.rs"), CandidateContext::Test);
        assert_eq!(classify("src\\main.rs"), CandidateContext::Product);
        assert_eq!(classify("src\\foo.test.js"), CandidateContext::Test);
    }

    #[test]
    fn nested_conventional_directories_are_classified() {
        assert_eq!(
            classify("packages/app/tests/integration.rs"),
            CandidateContext::Test
        );
        assert_eq!(
            classify("packages/app/examples/hello.rs"),
            CandidateContext::Example
        );
        assert_eq!(
            classify("packages/app/docs/guide.md"),
            CandidateContext::Doc
        );
    }

    #[test]
    fn case_insensitive_matching() {
        assert_eq!(classify("TESTS/foo.rs"), CandidateContext::Test);
        assert_eq!(classify("src/FOO.TEST.JS"), CandidateContext::Test);
        assert_eq!(classify("Docs/Readme.md"), CandidateContext::Doc);
    }
}
