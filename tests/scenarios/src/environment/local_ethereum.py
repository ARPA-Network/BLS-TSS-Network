import json
import os
from web3 import Web3


def some(message):
    print(message)

def get_abi(file_name):
    #read abi from json file
    with open(file_name, 'r') as f:
        abi = json.load(f)
        abi = abi["abi"]
    return abi

def get_contract(file_name, address):
    #get the contract object
    abi = get_abi(file_name)
    w3 = Web3(Web3.HTTPProvider('http://127.0.0.1:8545'))
    contract = w3.eth.contract(
        address = address,
        abi = abi
    )
    return contract

def to_checksum_address(lower_case_address):
    #Web3.py only accepts checksum addresses

    return Web3.toChecksumAddress(lower_case_address)

def contract_function_call(contract, function_name, *args):
    # contract is an instance of a web3 contract
    # function_name is a string representing the name of the function to execute
    # args is a variable-length argument list representing the parameters for the function
    # Get the function object from the contract instance
    function = getattr(contract.functions, function_name)
    # Call the function with the provided parameters
    result = function(*args).call()
    return result

def contract_function_transact(contract, function_name, *args):
    # contract is an instance of a web3 contract
    # function_name is a string representing the name of the function to execute
    # args is a variable-length argument list representing the parameters for the function
    
    # Get the function object from the contract instance
    function = getattr(contract.functions, function_name)
    
    # Call the function with the provided parameters
    result = function(*args).transact()
    
    return result

def deploy_controller():
    os.chdir("contracts")
    cmd = "forge script script/ControllerLocalTest.s.sol:ControllerLocalTestScript \
    --fork-url http://localhost:8545 \
    --broadcast"
    os.system(cmd)
    os.chdir("..")

