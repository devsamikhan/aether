#![allow(
    clippy::new_without_default,
    clippy::should_implement_trait,
    clippy::collapsible_if,
    clippy::unwrap_or_default,
    clippy::manual_memcpy,
    clippy::single_match
)]

pub mod codegen;
pub mod complexity;
pub mod crdt;
pub mod error;
pub mod interop;
pub mod lsp;
pub mod mcp_server;
pub mod module;
pub mod package_manager;
pub mod playground_server;
pub mod quantum;
pub mod syntax;
pub mod toolchain;
pub mod toolchain_templates;
pub mod tour;
pub mod types;
pub mod vm;
