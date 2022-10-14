pub mod adapter;
pub mod controller;
pub mod coordinator;
pub mod provider;

use ethers::prelude::*;
use ethers::providers::Http as HttpProvider;

pub(crate) type WalletSigner = SignerMiddleware<Provider<HttpProvider>, LocalWallet>;
