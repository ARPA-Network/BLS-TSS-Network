use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=proto");
    println!("cargo:rerun-if-changed=src/listener/test-contract");

    let mut prost_build = prost_build::Config::new();
    prost_build.btree_map(["members"]);
    fs::create_dir_all("./src/rpc_stub")?;
    let protos = &["proto/committer.proto", "proto/management.proto"];

    tonic_build::configure()
        .out_dir("./src/rpc_stub")
        .compile_with_config(prost_build, protos, &["proto"])?;

    #[cfg(feature = "unittest")]
    {
        println!("cargo:warning=Compiling test contracts...");
        compile_test_contracts()?;
    }

    Ok(())
}

#[cfg(feature = "unittest")]
fn compile_test_contracts() -> Result<(), Box<dyn std::error::Error>> {
    let contract_dir = PathBuf::from("./src/listener/test-contract");
    let output_dir = PathBuf::from("./src/test_contracts");
    
    if !contract_dir.exists() {
        println!("cargo:warning=Test contract directory not found, skipping contract compilation");
        return Ok(());
    }
    
    fs::create_dir_all(&output_dir)?;
    
    let solc_version = Command::new("solc")
        .arg("--version")
        .output();
    
    if solc_version.is_err() {
        println!("cargo:warning=solc not found in PATH. Please install the Solidity compiler.");
        return Err("solc not found in PATH".into());
    }
    
    let solc_unwrapped = solc_version.unwrap();
    let solc_version_output = String::from_utf8_lossy(&solc_unwrapped.stdout);
    println!("cargo:warning=Found solc: {}", solc_version_output.trim());
    
    let mut contract_files = Vec::new();
    for entry in fs::read_dir(&contract_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "sol") {
            contract_files.push(path);
        }
    }
    
    if contract_files.is_empty() {
        println!("cargo:warning=No Solidity contracts found in the test directory");
        return Ok(());
    }
    
    println!("cargo:warning=Found {} Solidity contracts", contract_files.len());
    
    let mut mod_file_content = String::from("// Auto-generated test contract modules\n\n");
    
    for contract_path in contract_files {
        let contract_name = contract_path
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        
        println!("cargo:warning=Compiling contract: {}", contract_name);
        
        let (json_data, _) = compile_solidity_contract(&contract_path)?;
        
        let contract_interfaces = extract_contract_interfaces(&contract_path)?;
        
        for interface_name in contract_interfaces {
            println!("cargo:warning=Generating bindings for: {}", interface_name);
            
            generate_rust_module(
                &output_dir,
                &interface_name,
                &contract_name,
                &json_data,
                ""
            )?;
            
            mod_file_content.push_str(&format!("pub mod {};\n", interface_name.to_lowercase()));
        }
    }
    
    let mut mod_file = fs::File::create(output_dir.join("mod.rs"))?;
    mod_file.write_all(mod_file_content.as_bytes())?;
    
    println!("cargo:warning=Test contracts compiled successfully");
    Ok(())
}

#[cfg(feature = "unittest")]
fn extract_contract_interfaces(contract_path: &Path) -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let output = Command::new("solc")
        .arg("--combined-json")
        .arg("abi")
        .arg(contract_path)
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(format!("Failed to get contract names: {}", error).into());
    }
    
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
        interfaces.push(contract_path.file_stem().unwrap().to_string_lossy().to_string());
    }
    
    println!("cargo:warning=Extracted interfaces from {}: {:?}", contract_path.display(), interfaces);
    
    Ok(interfaces)
}

#[cfg(feature = "unittest")]
fn compile_solidity_contract(contract_path: &Path) -> Result<(serde_json::Value, String), Box<dyn std::error::Error>> {
    println!("cargo:warning=Running solc on {}", contract_path.display());
    
    let output = Command::new("solc")
        .arg("--combined-json")
        .arg("abi,bin")
        .arg("--optimize")
        .arg(contract_path)
        .output()?;
    
    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        println!("cargo:warning=Solidity compilation error: {}", error);
        return Err(format!("Failed to compile Solidity contract: {}", error).into());
    }
    
    let json_output = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&json_output)?;
    
    if !parsed["contracts"].is_object() || parsed["contracts"].as_object().unwrap().is_empty() {
        return Err("No contracts found in solc output".into());
    }
    
    let default_bytecode = "".to_string();
    
    Ok((parsed, default_bytecode))
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
    
    let mut abi = "[]".to_string();
    let mut bytecode = "".to_string();
    
    if let Some(contracts) = json_data["contracts"].as_object() {
        for (key, value) in contracts {
            let parts: Vec<&str> = key.split(':').collect();
            if parts.len() == 2 && parts[1] == interface_name {
                abi = value["abi"].to_string();
                bytecode = value["bin"].as_str().unwrap_or("").to_string();
                println!("cargo:warning=Found exact match for {}: {} (ABI length: {})", 
                         interface_name, key, abi.len());
                break;
            }
        }
    }
    
    if abi == "[]" {
        if let Some(contracts) = json_data["contracts"].as_object() {
            for (key, value) in contracts {
                let parts: Vec<&str> = key.split(':').collect();
                if parts.len() == 2 && parts[1].contains(interface_name) {
                    abi = value["abi"].to_string();
                    bytecode = value["bin"].as_str().unwrap_or("").to_string();
                    println!("cargo:warning=Found partial match for {}: {} (ABI length: {})", 
                             interface_name, key, abi.len());
                    break;
                }
            }
        }
    }
    
    println!("cargo:warning=Using ABI with length {} for {}", abi.len(), interface_name);
    
    let rust_code = format!(
        r####"
// Auto-generated bindings for {interface_name} (from {source_name}.sol)
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

/// Deploy a new instance of the {interface_name} contract
pub async fn deploy_{snake_name}<M: Middleware + 'static>(
    client: Arc<M>
) -> Result<Address, Box<dyn std::error::Error>> {{
    let abi: ethers::abi::Abi = serde_json::from_str({upper_name}_ABI)?;
    
    let bytecode = {upper_name}_BYTECODE.parse::<Bytes>()?;
    
    let factory = ethers::contract::ContractFactory::new(abi, bytecode, client.clone());
    
    let deployer = factory.deploy(())?;
    
    let contract = deployer.send().await?;
    
    Ok(contract.address())
}}

/// Deploy a new instance of the {interface_name} contract with constructor arguments
pub async fn deploy_{snake_name}_with_args<M: Middleware + 'static, T: ethers::core::abi::Tokenize>(
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
        source_name = source_name,
        upper_name = interface_name.to_uppercase(),
        snake_name = convert_to_snake_case(interface_name),
        abi = abi,
        bytecode = bytecode,
    );
    
    let mut file = fs::File::create(file_path)?;
    file.write_all(rust_code.as_bytes())?;
    
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