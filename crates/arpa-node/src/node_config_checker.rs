use arpa_core::address_to_string;
use arpa_core::build_wallet_from_config;
use arpa_core::Config;
use ethers::signers::Signer;
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

    println!("{:?}", address_to_string(wallet.address()));

    Ok(())
}
