mod types;
mod utils;
use ethers_core::types::Address;
pub use types::*;
pub use utils::*;
pub mod log;

pub const PALCEHOLDER_ADDRESS: Address = Address::zero();
