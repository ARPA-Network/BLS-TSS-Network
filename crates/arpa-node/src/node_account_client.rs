use ethers::signers::LocalWallet;
use rand_chacha::rand_core::SeedableRng;
use std::path::PathBuf;
use structopt::StructOpt;

#[derive(StructOpt, Debug)]
#[structopt(name = "Arpa Node Account Generator")]
pub struct Opt {
    /// Mode to run.
    #[structopt(short = "m", long, possible_values = &["plain", "keystore", "hdwallet"])]
    mode: String,

    /// Seed
    #[structopt(short = "s", long)]
    seed: u64,

    /// Keystore password
    #[structopt(short = "w", long, required_if("mode", "keystore"))]
    password: Option<String>,

    /// Set the place to save keystore
    #[structopt(short = "p", long, parse(from_os_str), required_if("mode", "keystore"))]
    path: Option<PathBuf>,

    /// Set the name of keystore
    #[structopt(short = "n", long)]
    name: Option<String>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let opt = Opt::from_args();
    println!("{:#?}", opt);

    match opt.mode.as_str() {
        "keystore" => {
            let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(opt.seed);
            LocalWallet::new_keystore(
                opt.path.unwrap(),
                &mut rng,
                &opt.password.unwrap(),
                opt.name.as_deref(),
            )
            .unwrap();

            println!("keystore generated successfully.");
        }
        _ => panic!("not implemented."),
    }

    Ok(())
}
