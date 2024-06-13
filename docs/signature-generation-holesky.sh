#!/bin/bash
# Parameters needed

avs_directory_address="0x055733000064333CaDDbC92763c58BF0192fFeBf" # Eigen AVS address on holesky testnet
function_signature="calculateOperatorAVSRegistrationDigestHash(address,address,bytes32,uint256)(bytes32)"
operator_address= "<your operator address>"
avs_address= "0xd36b6E5eEe8311d7Bffb2f3Bb33301A1AB7De101" # Our AVS contract address
salt= "<your salt value>" #example: "0x4d4b520000000000000000000000000000000000000000000000000000000000", just make sure you pick some random value that wasn’t used previously
expiry= "<your expiry value>" #example: "134234235" This is the expiration time for your signature.
rpc_url= "<your holesky rpc endpoint url>"

# Call calculateOperatorAVSRegistrationDigestHash function provided by eigenlayer contract

echo "Calling calculateOperatorAVSRegistrationDigestHash function..."
digest_hash=$(cast call $avs_directory_address "$function_signature" $operator_address $avs_address $salt $expiry --rpc-url $rpc_url)
echo "Digest hash: $digest_hash"

# with the digest hash it returns, call wallet sign to get your signature
# note: 
# 1. The "--no-hash" tag is essential for this signature to work
# 2. Original issue reference: https://github.com/foundry-rs/foundry/issues/6794
echo "Signing digest hash..."
signature=$(cast wallet sign $digest_hash --interactive --no-hash)
echo "Signature: $signature"
