pub use arpa_node_log_impl::*;
pub use log::debug;
pub use log_mdc;

#[derive(Debug)]
pub struct LogModel<'a> {
    pub fn_name: &'a str,
    pub fn_args: &'a [&'a str],
    pub fn_return: &'a str,
}
