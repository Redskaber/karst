//! Cross-crate integration test single entry
//! Selective run: `cargo test --test runner <module>::<case>`.

#[path = "common/mod.rs"]
mod common;

#[path = "v0/stage0/plan/span_seed_tests.rs"]
mod span_seed_tests;

#[path = "v0/stage0/plan/core_seed_tests.rs"]
mod core_seed_tests;

#[path = "v0/stage0/plan/syntax_seed_tests.rs"]
mod syntax_seed_tests;

#[path = "v0/stage0/plan/reader_seed_tests.rs"]
mod reader_seed_tests;

#[path = "v0/stage0/plan/expander_seed_tests.rs"]
mod expander_seed_tests;
