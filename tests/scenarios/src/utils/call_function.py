"""
Util functions to get event
"""
from web3 import Web3
import json

def get_abi(file_name):
    """
    Read an ABI (Application Binary Interface) string from a JSON file.
    """
    with open(file_name, 'r', encoding='utf-8') as file:
        abi = json.load(file)
        abi = abi["abi"]
    return abi

CONTRACT_ABI = get_abi('contracts/out/AdvancedGetShuffledArrayExampleTest.sol/AdvancedGetShuffledArrayExampleTest.json')

# Replace the placeholder below with your contract address
CONTRACT_ADDRESS = '0xa3Fc37b87CbB7D12f7b8d650B544Af1A7B1985AE'

def main():
    # Connect to Sepolia Testnet via Infura
    w3 = Web3(Web3.HTTPProvider(f'https://eth-sepolia.g.alchemy.com/v2/7RFds6TGJiIN5gR4BHrEtiMjJRvV44Na'))

    # Set the defalut account from private key
    w3.eth.default_account = '0x6a129A0C886bC8Ac6D0b4e1C3D460498345d534d'
    # Set up the contract instance
    contract_instance = w3.eth.contract(address=Web3.to_checksum_address(CONTRACT_ADDRESS), abi=CONTRACT_ABI)

    result = contract_instance.functions.getRandomNumberThenGenerateShuffledArray(10, 2, 42, 0, 350000, 1000000000).transact()
    tx_receipt = w3.eth.wait_for_transaction_receipt(result)

    print(f"Event Logs: {result}")

if __name__ == '__main__':
    main()
