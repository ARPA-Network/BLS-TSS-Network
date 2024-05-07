use arpa_core::build_wallet_from_config;
use arpa_core::Config;
use ethers::signers::Signer;
use ethers::types::Address;
use ethers::utils::keccak256;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "Arpa Config Checker")]
pub struct Opt {
    /// Set the config path
    #[structopt(
        short = "c",
        long,
        parse(from_os_str),
        default_value = "crates/arpa-node/conf/config.yml"
    )]
    config_path: PathBuf,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();

    let config = Config::load(opt.config_path);

    let wallet = build_wallet_from_config(config.get_account())?;

    println!("{:?}", to_checksum(&wallet.address(), None));

    Ok(())
}

/// Converts an Ethereum address to the checksum encoding
/// Ref: <https://github.com/ethereum/EIPs/blob/master/EIPS/eip-55.md>
pub fn to_checksum(addr: &Address, chain_id: Option<u8>) -> String {
    let prefixed_addr = match chain_id {
        Some(chain_id) => format!("{}0x{:x}", chain_id, addr),
        None => format!("{:x}", addr),
    };
    let hash = hex::encode(keccak256(&prefixed_addr));
    let hash = hash.as_bytes();

    let addr_hex = hex::encode(addr.as_bytes());
    let addr_hex = addr_hex.as_bytes();

    addr_hex
        .iter()
        .zip(hash)
        .fold("0x".to_owned(), |mut encoded, (addr, hash)| {
            encoded.push(if *hash >= 56 {
                addr.to_ascii_uppercase() as char
            } else {
                addr.to_ascii_lowercase() as char
            });
            encoded
        })
}
