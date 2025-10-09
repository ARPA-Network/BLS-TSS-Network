use std::fs;
#[cfg(feature = "unittest")]
use std::io::Write;
#[cfg(feature = "unittest")]
use std::path::{Path, PathBuf};
#[cfg(feature = "unittest")]
use std::process::Command;

const RPC_STUB_DIR: &str = "./src/rpc_stub";
const PROTO_DIR: &str = "proto";
#[cfg(feature = "unittest")]
const SOLIDITY_EXTENSION: &str = "sol";
#[cfg(feature = "unittest")]
const OUTPUT_DIR: &str = "./src/test_contracts";
#[cfg(feature = "unittest")]
const CONTRACT_DIR: &str = "./src/listener/test-contract";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=src/listener/test-contract");

    let mut prost_build = tonic_prost_build::Config::new();
    prost_build.btree_map(["members"]);
    fs::create_dir_all(RPC_STUB_DIR)?;
    let protos = &["proto/committer.proto", "proto/management.proto"];

    tonic_prost_build::configure()
        .out_dir(RPC_STUB_DIR)
        .compile_with_config(prost_build, protos, &[PROTO_DIR])?;

    #[cfg(feature = "unittest")]
    {
        cargo_warning("Compiling test contracts...");
        compile_test_contracts()?;
    }

    Ok(())
}

#[cfg(feature = "unittest")]
fn cargo_warning(msg: &str) {
    println!("cargo:warning={}", msg);
}

#[cfg(feature = "unittest")]
fn execute_solc_command(args: &[&str]) -> Result<std::process::Output, Box<dyn std::error::Error>> {
    let output = Command::new("solc").args(args).output()?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("solc command failed: {}", error).into());
    }

    Ok(output)
}

#[cfg(feature = "unittest")]
fn write_file_content(path: &Path, content: &str) -> Result<(), Box<dyn std::error::Error>> {
    let mut file = fs::File::create(path)?;
    file.write_all(content.as_bytes())?;
    Ok(())
}

#[cfg(feature = "unittest")]
fn find_contract_abi(
    contracts: &serde_json::Map<String, serde_json::Value>,
    interface_name: &str,
) -> Option<(String, String)> {
    for (key, value) in contracts {
        if let Some(contract_name) = key.split(':').last() {
            if contract_name == interface_name {
                let abi = value["abi"].to_string();
                let bytecode = value["bin"].as_str().unwrap_or("").to_string();
                cargo_warning(&format!(
                    "Found exact match for {}: {} (ABI length: {})",
                    interface_name,
                    key,
                    abi.len()
                ));
                return Some((abi, bytecode));
            }
        }
    }

    for (key, value) in contracts {
        if let Some(contract_name) = key.split(':').last() {
            if contract_name.contains(interface_name) {
                let abi = value["abi"].to_string();
                let bytecode = value["bin"].as_str().unwrap_or("").to_string();
                cargo_warning(&format!(
                    "Found partial match for {}: {} (ABI length: {})",
                    interface_name,
                    key,
                    abi.len()
                ));
                return Some((abi, bytecode));
            }
        }
    }

    None
}

#[cfg(feature = "unittest")]
fn generate_deploy_functions(interface_name: &str, snake_name: &str, upper_name: &str) -> String {
    format!(
        r####"
/// Deploy a new instance of the {interface_name} contract
pub async fn deploy_{snake_name}<M: Middleware + 'static>(
    client: Arc<M>
) -> Result<Address, Box<dyn std::error::Error>> {{
    deploy_contract_internal(client, ()).await
}}

/// Deploy a new instance of the {interface_name} contract with constructor arguments
pub async fn deploy_{snake_name}_with_args<M: Middleware + 'static, T: ethers::core::abi::Tokenize>(
    client: Arc<M>,
    args: T
) -> Result<Address, Box<dyn std::error::Error>> {{
    deploy_contract_internal(client, args).await
}}

async fn deploy_contract_internal<M: Middleware + 'static, T: ethers::core::abi::Tokenize>(
    client: Arc<M>,
    args: T
) -> Result<Address, Box<dyn std::error::Error>> {{
    let abi: ethers::abi::Abi = serde_json::from_str({upper_name}_ABI)?;
    let bytecode = {upper_name}_BYTECODE.parse::<Bytes>()?;
    let factory = ethers::contract::ContractFactory::new(abi, bytecode, client.clone());
    let deployer = factory.deploy(args)?;
    let contract = deployer.send().await?;
    Ok(contract.address())
}}

/// Get a handle to a deployed contract instance
pub fn get_{snake_name}_at<M: Middleware>(
    address: Address,
    client: Arc<M>
) -> {interface_name}<M> {{
    {interface_name}::new(address, client)
}}

/// Deploy and get a typed contract instance
pub async fn deploy_and_get_{snake_name}<M: Middleware + 'static>(
    client: Arc<M>
) -> Result<{interface_name}<M>, Box<dyn std::error::Error>> {{
    let address = deploy_{snake_name}(client.clone()).await?;
    Ok(get_{snake_name}_at(address, client))
}}

/// Deploy with args and get a typed contract instance
pub async fn deploy_with_args_and_get_{snake_name}<M: Middleware + 'static, T: ethers::core::abi::Tokenize>(
    client: Arc<M>,
    args: T
) -> Result<{interface_name}<M>, Box<dyn std::error::Error>> {{
    let address = deploy_{snake_name}_with_args(client.clone(), args).await?;
    Ok(get_{snake_name}_at(address, client))
}}
"####,
        interface_name = interface_name,
        snake_name = snake_name,
        upper_name = upper_name
    )
}

#[cfg(feature = "unittest")]
fn generate_contract_module(
    interface_name: &str,
    source_name: &str,
    abi: &str,
    bytecode: &str,
) -> String {
    let upper_name = interface_name.to_uppercase();
    let snake_name = convert_to_snake_case(interface_name);

    let deploy_functions = generate_deploy_functions(interface_name, &snake_name, &upper_name);

    format!(
        r####"// Auto-generated bindings for {interface_name} (from {source_name}.sol)
use ethers::prelude::*;
use std::sync::Arc;

/// ABI for the {interface_name} contract
pub const {upper_name}_ABI: &str = r##"{abi}"##;

/// Bytecode for the {interface_name} contract
pub const {upper_name}_BYTECODE: &str = r##"{bytecode}"##;

abigen!(
    {interface_name},
    r##"{abi}"##,
);
{deploy_functions}
"####,
        interface_name = interface_name,
        source_name = source_name,
        upper_name = upper_name,
        abi = abi,
        bytecode = bytecode,
        deploy_functions = deploy_functions
    )
}

#[cfg(feature = "unittest")]
fn compile_test_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let contract_dir = PathBuf::from(CONTRACT_DIR);
    let output_dir = PathBuf::from(OUTPUT_DIR);

    if !contract_dir.exists() {
        cargo_warning("Test contract directory not found, skipping contract compilation");
        return Ok(());
    }

    fs::create_dir_all(&output_dir)?;

    check_solc_version()?;

    let contract_files = collect_contract_files(&contract_dir)?;

    if contract_files.is_empty() {
        cargo_warning("No Solidity contracts found in the test directory");
        return Ok(());
    }

    cargo_warning(&format!(
        "Found {} Solidity contracts",
        contract_files.len()
    ));

    let mut mod_file_content = String::from("// Auto-generated test contract modules\n\n");

    for contract_path in contract_files {
        let contract_name = contract_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();

        cargo_warning(&format!("Compiling contract: {}", contract_name));

        let (json_data, _) = compile_solidity_contract(&contract_path)?;
        let contract_interfaces = extract_contract_interfaces(&contract_path)?;

        for interface_name in contract_interfaces {
            cargo_warning(&format!("Generating bindings for: {}", interface_name));

            generate_rust_module(&output_dir, &interface_name, &contract_name, &json_data, "")?;

            mod_file_content.push_str(&format!("pub mod {};\n", interface_name.to_lowercase()));
        }
    }

    write_file_content(&output_dir.join("mod.rs"), &mod_file_content)?;
    cargo_warning("Test contracts compiled successfully");
    Ok(())
}

#[cfg(feature = "unittest")]
fn check_solc_version() -> Result<(), Box<dyn std::error::Error>> {
    let output = Command::new("solc")
        .arg("--version")
        .output()
        .map_err(|_| "solc not found in PATH. Please install the Solidity compiler.")?;

    if !output.status.success() {
        return Err("Failed to get solc version".into());
    }

    let version_output = String::from_utf8_lossy(&output.stdout);
    cargo_warning(&format!("Found solc: {}", version_output.trim()));
    Ok(())
}

#[cfg(feature = "unittest")]
fn collect_contract_files(contract_dir: &Path) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    let mut contract_files = Vec::new();

    for entry in fs::read_dir(contract_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .extension()
                .map_or(false, |ext| ext == SOLIDITY_EXTENSION)
        {
            contract_files.push(path);
        }
    }

    Ok(contract_files)
}

#[cfg(feature = "unittest")]
fn extract_contract_interfaces(
    contract_path: &Path,
) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output =
        execute_solc_command(&["--combined-json", "abi", &contract_path.to_string_lossy()])?;

    let json_output = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&json_output)?;

    let mut interfaces = Vec::new();

    if let Some(contracts) = parsed["contracts"].as_object() {
        for key in contracts.keys() {
            if let Some(contract_name) = key.split(':').last() {
                if !interfaces.contains(&contract_name.to_string()) {
                    interfaces.push(contract_name.to_string());
                }
            }
        }
    }

    if interfaces.is_empty() {
        interfaces.push(
            contract_path
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .to_string(),
        );
    }

    cargo_warning(&format!(
        "Extracted interfaces from {}: {:?}",
        contract_path.display(),
        interfaces
    ));

    Ok(interfaces)
}

#[cfg(feature = "unittest")]
fn compile_solidity_contract(
    contract_path: &Path,
) -> Result<(serde_json::Value, String), Box<dyn std::error::Error>> {
    cargo_warning(&format!("Running solc on {}", contract_path.display()));

    let output = execute_solc_command(&[
        "--combined-json",
        "abi,bin",
        "--optimize",
        &contract_path.to_string_lossy(),
    ])?;

    let json_output = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&json_output)?;

    if !parsed["contracts"].is_object() || parsed["contracts"].as_object().unwrap().is_empty() {
        return Err("No contracts found in solc output".into());
    }

    Ok((parsed, String::new()))
}

#[cfg(feature = "unittest")]
fn generate_rust_module(
    output_dir: &Path,
    interface_name: &str,
    source_name: &str,
    json_data: &serde_json::Value,
    _bytecode: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let module_name = interface_name.to_lowercase();
    let file_path = output_dir.join(format!("{}.rs", module_name));

    let (abi, bytecode) = if let Some(contracts) = json_data["contracts"].as_object() {
        find_contract_abi(contracts, interface_name)
            .unwrap_or_else(|| ("[]".to_string(), String::new()))
    } else {
        ("[]".to_string(), String::new())
    };

    cargo_warning(&format!(
        "Using ABI with length {} for {}",
        abi.len(),
        interface_name
    ));

    let rust_code = generate_contract_module(interface_name, source_name, &abi, &bytecode);
    write_file_content(&file_path, &rust_code)?;

    Ok(())
}

#[cfg(feature = "unittest")]
fn convert_to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if c.is_uppercase() {
            if i > 0 {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap());
        } else {
            result.push(c);
        }
    }
    result
}
