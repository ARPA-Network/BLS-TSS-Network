*** Settings ***
Documentation       This resource file sets the constants and variables
...                 that applies to all test suites and test cases

Library             Process
Library             String
Library             OperatingSystem


*** Variables ***
# Ethereum Mainnet Sanity:
${TEN_MILLION_BLOCKS}                   10000000
${WRAPPED_ETHER_CONTRACT_ADDRESS}       0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2

# ARPA ERC20 information:
${FORK_RPC_URL}                         https://mainnet.infura.io/v3/e263f48ae1f545198575c7c7d4088f57
${ARPA_HOLDER_ADDRESS}                  0xf977814e90da44bfa03b6295a0616a897441acec
${ARPA_CONTRACT_ADDRESS}                0xBA50933C268F567BDC86E1aC131BE072C6B0b71a
${ACCOUNT_PATH}                         ACCOUNT_PATH
${NUM_ACCOUNTS}                         20
${BALANCE}                              1000

# Ethereum RPC client, Cast CLI command and arguments:
${ARGS_IMPERSONATE_ACCOUNT}             rpc anvil_impersonateAccount
${ARGS_STOP_IMPERSONATE_ACCOUNT}        rpc anvil_stopImpersonatingAccount
${ARGS_TRANSFER_ARPA}                   send ${ARPA_CONTRACT_ADDRESS}
...                                     transfer(address,uint256)
...                                     --from=${ARPA_HOLDER_ADDRESS}
${ARGS_GET_ARPA_BALANCE}                call ${ARPA_CONTRACT_ADDRESS} balanceOf(address)


*** Keywords ***
Setup Scenario Testing Environment
    [Documentation]    Sets up the scenario testing encironment
    Start Local Ethereum Chain
    Local Ethereum Chain Should Be Up And Running
    Get Latest Block Number Should Work
    Mine Blocks Should Work
    Get Balance Should Work
    Set Balance Should Work

Teardown Scenario Testing Environment
    [Documentation]    Tears down the scenario testing environment
    Stop Local Ethereum Chain

Process Completes Successfully
    [Documentation]    The CLI command issued executes and terminates without error
    [Arguments]    ${result}
    Should Be Equal As Integers    ${result.rc}    0
    Should Be Empty    ${result.stderr}

# Helper function section for interacting with local Ethereum chain

Start Local Ethereum Chain
    [Documentation]    Starts a local Ethereum blockchain child process in the background with Anvil CLI
    ...    then saves the process handle to a suite-wide variable for later termination in the suite teardown

    ${anvil_process_handle} =    Start Process    anvil
    ...    --fork-url    ${FORK_RPC_URL}
    ...    --accounts    ${NUM_ACCOUNTS}
    ...    --balance    ${BALANCE}

    Process Should Be Running    ${anvil_process_handle}
    Set Global Variable    $LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE    ${anvil_process_handle}

    # Temporary solution here
    # not robust enough since the kickoff time of Anvil can vary depending on the hardware and network
    # A better solution would be to poll the stdout and inspect the output to ensure Anvil is properly running
    Sleep    20s

Stop Local Ethereum Chain
    [Documentation]    Stops the local Ethereum blockchain child process running in the backgroud of Anvil
    ...    by grabbing the previously saved process handle during the suite setup process
    Terminate Process    ${LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE}
    Process Should Be Stopped    ${LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE}

Get Chain ID
    [Documentation]    Tests the local Ethereum chain is up and running
    ${result} =    Run Process    cast    chain-id
    Process Completes Successfully    ${result}
    RETURN    ${result.stdout}

Get Latest Block Number
    [Documentation]    Gets the latest block number
    ${result} =    Run Process    cast    block-number
    Process Completes Successfully    ${result}
    RETURN    ${result.stdout}

Mine Blocks
    [Documentation]    Mines n number of blocks
    [Arguments]    ${num_blocks}
    Process Should Be Running    ${LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE}
    ${result} =    Run Process    cast    rpc    anvil_mine    ${num_blocks}
    Process Completes Successfully    ${result}

Set Balance
    [Documentation]    Sets the ETH balance of a given address
    [Arguments]    ${address}    ${amount}
    Process Should Be Running    ${LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE}
    ${result} =    Run Process    cast    rpc    anvil_setBalance    ${address}    ${amount}
    Process Completes Successfully    ${result}

Get Balance
    [Documentation]    Gets the ETH balance of a given address
    [Arguments]    ${address}
    Process Should Be Running    ${LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE}
    ${result} =    Run Process    cast    balance    ${address}
    Process Completes Successfully    ${result}
    RETURN    ${result.stdout}

# Sanity check section for local Ethereum chain and helper functions:

Local Ethereum Chain Should Be Up And Running
    [Documentation]    Sanity check
    ...    to ensure the local Ethereum chain is properly forked from the Mainnet
    ...    and ready to accept RPC calls
    ${chain_id} =    Get Chain ID
    Should Be True    ${chain_id} > 0

Get Latest Block Number Should Work
    [Documentation]    Test Get Latest Block Number
    ${block_number} =    Get Latest Block Number
    Should Be True    ${block_number} > ${TEN_MILLION_BLOCKS}

Mine Blocks Should Work
    [Documentation]    Test Mine Blocks
    ${previous_block_number} =    Get Latest Block Number
    Mine Blocks    1
    ${current_block_number} =    Get Latest Block Number
    Should Be True    ${previous_block_number} + 1 == ${current_block_number}

Get Balance Should Work
    [Documentation]    Test Get Balance
    ${balance} =    Get Balance    ${ARPA_HOLDER_ADDRESS}
    Should Be True    ${balance} > 0

Set Balance Should Work
    [Documentation]    Test Set Balance
    Set Balance    ${WRAPPED_ETHER_CONTRACT_ADDRESS}    0
    ${balance} =    Get Balance    ${WRAPPED_ETHER_CONTRACT_ADDRESS}
    Should Be Equal    ${balance}    0

# TODO: Keywords to interact with ERC-20 token contract
# TODO: Keywords to build & deploy ARPA smart contracts
# TODO: Keywords to interact with ARPA smart contracts
# TODO: Keywords to build & run nodes
# TODO: Keywords to interact with nodes via RPC
# TODO: Test Cases for node registration
