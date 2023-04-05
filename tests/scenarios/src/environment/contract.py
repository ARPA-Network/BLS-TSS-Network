"""
This module contains necessary packages to interact with web3 and blockchain 
"""
import json
import os
import subprocess
import sys
from web3 import Web3

def get_abi(file_name):
    """
    Read an ABI (Application Binary Interface) string from a JSON file.
    """
    with open(file_name, 'r', encoding='utf-8') as file:
        abi = json.load(file)
        abi = abi["abi"]
    return abi

def get_contract(file_name, address):
    """
    Get the contract object.
    """
    abi = get_abi(file_name)
    web3 = Web3(Web3.HTTPProvider('http://127.0.0.1:8545'))
    contract = web3.eth.contract(
        address = address,
        abi = abi
    )
    return contract

def to_checksum_address(lower_case_address):
    """
    Web3.py only accepts checksum addresses
    To be removed, new version of web3.py will accept lower case addresses
    """
    return Web3.to_checksum_address(lower_case_address)

def contract_function_call(contract, function_name, *args):
    """
    contract is an instance of a web3 contract
    function_name is a string representing the name of the function to execute
    args is a variable-length argument list representing the parameters for the function
    Get the function object from the contract instance
    """
    function = getattr(contract.functions, function_name)
    # Call the function with the provided parameters
    result = function(*args).call()
    return result

def contract_function_transact(contract, function_name, *args):
    """
    contract is an instance of a web3 contract
    function_name is a string representing the name of the function to execute
    args is a variable-length argument list representing the parameters for the function
    
    Get the function object from the contract instance
    """
    function = getattr(contract.functions, function_name)
    # Call the function with the provided parameters
    result = function(*args).transact()
    return result

def exec_script(script_name):
    """
    Executes a script from the "contracts/script" directory.
    """
    os.chdir("contracts")
    cmd = ("forge script script/" + script_name
        + " --fork-url http://localhost:8545 --broadcast --slow --revert-strings debug")
    os.system(cmd)
    os.chdir("..")


def deploy_controller():
    """
    Deploy controller contract. becuase the controller contract is too large,
    need to input 'y' to continue.
    """
    cmd = ["forge", "script", "script/DeployControllerLocalTest.s.sol:DeployControllerTestScript",
           "--fork-url", "http://localhost:8545", "--broadcast", "--slow", "--revert-strings", "debug"]
    pty_cmd = cmd = [sys.executable,
       '-c',
       'import pty, sys; pty.spawn(sys.argv[1:])',
       *cmd]
    proc = subprocess.Popen(
        pty_cmd,
        stdin=subprocess.PIPE,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        cwd="contracts",
    )
    out = proc.communicate(b"y")[0]
    print(out.decode('utf-8'))
    proc.wait()

def get_contract_address_from_file(file_name):
    """
    Get contract address from receipt file.
    file_name: the receipt file name.
    """
    with open(file_name, 'r', encoding='UTF-8') as f:
        data = json.load(f)
    transactions = data.get('transactions')
    contract_addresses = []
    for transaction in transactions:
        contract_address = transaction.get('contractAddress')
        if contract_address is not None:
            contract_addresses.append(contract_address)
    return contract_addresses
