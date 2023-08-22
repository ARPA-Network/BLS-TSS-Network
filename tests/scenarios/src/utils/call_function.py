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

# Replace with your contract address
CONTRACT_ADDRESS = 'xxx'

def main():
    # Connect to Sepolia
    w3 = Web3(Web3.HTTPProvider('https://eth-sepolia.g.alchemy.com/v2/xxx'))
    # Set the defalut account
    w3.eth.default_account = ''
    # Set up the contract instance
    contract_instance = w3.eth.contract(address=Web3.to_checksum_address(CONTRACT_ADDRESS), abi=CONTRACT_ABI)
    # Call the function
    result = contract_instance.functions.getRandomNumberThenGenerateShuffledArray(10, 2, 42, 0, 350000, 1000000000).transact()
    tx_receipt = w3.eth.wait_for_transaction_receipt(result)

    print(f"Event Logs: {result}")

if __name__ == '__main__':
    main()
