use chrono::Local;
use ethers::{
    prelude::k256::ecdsa::SigningKey,
    signers::{coins_bip39::English, LocalWallet, MnemonicBuilder, Wallet},
    types::Address,
};
use std::env;

use super::{context::types::Account, error::ConfigError};

pub fn format_now_date() -> String {
    let fmt = "%Y-%m-%d %H:%M:%S";
    Local::now().format(fmt).to_string()
}

pub fn address_to_string(address: Address) -> String {
    format!("{:?}", address)
}

pub fn build_wallet_from_config(account: Account) -> Result<Wallet<SigningKey>, ConfigError> {
    if account.hdwallet.is_some() {
        let mut hd = account.hdwallet.unwrap();
        if hd.mnemonic.eq("env") {
            hd.mnemonic = env::var("ARPA_NODE_HD_ACCOUNT_MNEMONIC")?;
        }
        let mut wallet = MnemonicBuilder::<English>::default().phrase(&*hd.mnemonic);

        if hd.path.is_some() {
            wallet = wallet.derivation_path(&hd.path.unwrap()).unwrap();
        }
        if hd.passphrase.is_some() {
            wallet = wallet.password(&hd.passphrase.unwrap());
        }
        return Ok(wallet.index(hd.index).unwrap().build()?);
    } else if account.keystore.is_some() {
        let mut keystore = account.keystore.unwrap();
        if keystore.password.eq("env") {
            keystore.password = env::var("ARPA_NODE_ACCOUNT_KEYSTORE_PASSWORD")?;
        }
        return Ok(LocalWallet::decrypt_keystore(
            &keystore.path,
            &keystore.password,
        )?);
    } else if account.private_key.is_some() {
        let mut private_key = account.private_key.unwrap();
        if private_key.eq("env") {
            private_key = env::var("ARPA_NODE_ACCOUNT_PRIVATE_KEY")?;
        }
        return Ok(private_key.parse::<Wallet<SigningKey>>()?);
    }

    Err(ConfigError::LackOfAccount)
}

#[cfg(test)]
pub mod util_tests {
    use ethers::types::Address;

    use crate::node::utils::format_now_date;

    #[test]
    fn test_format_now_date() {
        let res = format_now_date();
        println!("{:?}", res);
    }

    #[test]
    fn test_address_format() {
        let good_address_in_str = "0x0000000000000000000000000000000000000001";
        let address = good_address_in_str.parse::<Address>();
        assert!(address.is_ok());
        let bad_address_in_str = "0x1";
        let address = bad_address_in_str.parse::<Address>();
        assert!(address.is_err());
    }
}
