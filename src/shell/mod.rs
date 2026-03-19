//! Interactive shell module for vfs.
//!
//! Provides a REPL (Read-Eval-Print-Loop) experience with tab completion
//! and command history.

mod completion;
mod repl;

pub use repl::Shell;
