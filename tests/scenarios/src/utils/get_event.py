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

CONTRACT_ABI = get_abi('contracts/out/RewardLib.sol/RewardLib.json')

# Replace the placeholder below with your contract address
CONTRACT_ADDRESS = '0xCf7Ed3AccA5a467e9e704C703E8D87F634fB0Fc9'

def main():
    """
    This function connects to Sepolia Testnet via Infura.
    """
    # Connect to Sepolia Testnet via Infura
    #w3 = Web3(Web3.HTTPProvider(f'https://eth-sepolia.g.alchemy.com/v2/7RFds6TGJiIN5gR4BHrEtiMjJRvV44Na'))
    w3 = Web3(Web3.HTTPProvider('http://127.0.0.1:8545'))

    # Set up the contract instance
    contract_instance = w3.eth.contract(address=Web3.to_checksum_address(CONTRACT_ADDRESS), abi=CONTRACT_ABI)

    # Define the event name and start block
    #event_name = 'SubscriptionReferralSet'
    #event_name = 'RandomnessRequestResult'
    #event_name = 'SubscriptionReferralSet'
    event_name = 'RewardAdded'
    start_block = 3626150

    # Get the event
    event = contract_instance.events[event_name]
    logs = event.create_filter(fromBlock=start_block).get_all_entries()

    print(f"Event Logs: {logs}")

if __name__ == '__main__':
    main()
