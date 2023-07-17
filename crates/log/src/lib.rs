//! As an attribute macro, log_function will
//! 1. Automatically log the name, input and return value of current function at debug level
//!  before it returns by trying to recognize return stmt and inserting a `debug!` stmt.
//! 2. Set the name of current function as a key in `mdc` at the beginning of the function and
//!  remove it before the function returns.
//!
//! Note:
//! 1. Input and return type need to implement `Debug`.
//! 2. When dealing with async function, using `#![feature(async_fn_in_trait)]` is recommended.
//! Also this is compatible with `#[async_trait]`.
//! 3. To protect secrets, input and return values are ignored by default.
//! You can specify whether to print all values, or a subset of them with semantic literal options.
//! For example:
//!     ```
//!     use arpa_log::*;
//!
//!     #[log_function("show-input", "except foo bar", "show-return")]
//!     fn show_subset_of_input_and_return_value(foo: usize, bar: usize, baz: usize) -> usize {
//!        foo + bar + baz
//!     }
//!     ```
//!     Then the log should be: {"message":"LogModel { fn_name: \"show_subset_of_input_and_return_value\",
//!     fn_args: [\"foo: ignored\", \"bar: ignored\", \"baz: 3\"], fn_return: \"6\" }","level":"DEBUG",
//!     "target":"show_subset_of_input_and_return_value","mdc":{"fn_name":"show_subset_of_input_and_return_value"}}
//!     with test logger.
//!
//! Note: Logging result can be different with different logger implementation.

pub use arpa_log_impl::*;
pub use log::debug;
pub use log_mdc;

#[derive(Debug)]
pub struct LogModel<'a> {
    pub fn_name: &'a str,
    pub fn_args: &'a [&'a str],
    pub fn_return: &'a str,
}
