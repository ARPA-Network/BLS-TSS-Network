*** Settings ***
Documentation       This resource file sets the constants and variables
...                 that applies to all test suites and test cases

Library             Process
Library             String
Library             OperatingSystem
Library             environment/node.py
Library             environment/util.py

*** Variables ***
${NODE_ACCOUNT_LIST}
${NODE_PROCESS_LIST}
${EMPTY_LIST}
${LOG_PATH}                             crates/arpa-node/log/running/
${ARPA_HOLDER_ADDRESS}                  0xf977814e90da44bfa03b6295a0616a897441acec
${ARPA_CONTRACT_ADDRESS}                0xBA50933C268F567BDC86E1aC131BE072C6B0b71a                
${NUM_ACCOUNTS}                         40
${BALANCE}                              1000
${ENV_PATH}                             tests/scenarios/.env
${CONTRACT_PATH}                        contracts/

# Ethereum RPC client, Cast CLI command and arguments:
${LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE}
${BLOCK_TIME}                           0

*** Keywords ***

Setup Chain Environment
    [Documentation]    Sets up the scenario testing encironment
    Source Environment Variables
    Start Local Ethereum Chain
    Local Ethereum Chain Should Be Up And Running

Teardown Scenario Testing Environment
    [Documentation]    Tears down the scenario testing environment
    Stop Local Ethereum Chain
    Terminate All Processes
    ${list_len} =   Get Length    ${NODE_PROCESS_LIST}
    Kill Previous Node    ${list_len}
    Set Global Variable    $NODE_PROCESS_LIST    ${EMPTY_LIST}
    Set Global Variable    $BLOCK_TIME    0
    Set Global Variable    $USER_CONTRACT_ADDRESS    NULL
    Clear Database
    

Process Completes Successfully
    [Documentation]    The CLI command issued executes and terminates without error
    [Arguments]    ${result}
    Should Be Equal As Integers    ${result.rc}    0
    Should Be Empty    ${result.stderr}

Start Local Ethereum Chain
    [Documentation]    Starts a local Ethereum blockchain child process in the background with Anvil CLI
    ...    then saves the process handle to a suite-wide variable for later termination in the suite teardown
    Kill Process By Name    anvil
    IF  ${BLOCK_TIME} == 0
        ${anvil_process_handle} =    Start Process    anvil
        ...    --accounts    ${NUM_ACCOUNTS}
        ...    --gas-price    100000000
        ...    stdout=${LOG_PATH}/anvil-chain.log
    ELSE
        ${anvil_process_handle} =    Start Process    anvil
        ...    --block-time    ${BLOCK_TIME}
        ...    --accounts    ${NUM_ACCOUNTS}
        ...    --gas-price    100000000
        ...    stdout=${LOG_PATH}/anvil-chain.log
    END

    Process Should Be Running    ${anvil_process_handle}
    Set Global Variable    $LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE    ${anvil_process_handle}
    Sleep    2s
    ${result} =    Parse Chain Result To Account List
    Set Global Variable    $NODE_ACCOUNT_LIST    ${result}

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

Get Timestamp
    [Documentation]    Gets current block timestamp
    ${result} =    Run Process    cast    age
    Process Completes Successfully    ${result}
    RETURN    ${result.stdout}

Set Timestamp Interval
    [Documentation]    Sets next block timestamp interval
    [Arguments]    ${interval}
    ${result} =    Run Process    cast    rpc    anvil_setBlockTimestampInterval    ${interval}
    Process Completes Successfully    ${result}

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

Get ARPA Balance
    [Documentation]    Gets the ARPA balance of a given address
    [Arguments]    ${address}
    ${result} =    Run Process    cast    call    ${ARPA_CONTRACT_ADDRESS}
    ...    balanceOf(address)(uint256)    ${address}    
    Process Completes Successfully    ${result}
    RETURN    ${result.stdout}

Get Private Key By Index
    [Arguments]    ${node_index}
    RETURN    ${NODE_ACCOUNT_LIST[${node_index} - 1].key}

Local Ethereum Chain Should Be Up And Running
    [Documentation]    Sanity check
    ...    to ensure the local Ethereum chain is properly forked from the Mainnet
    ...    and ready to accept RPC calls
    ${chain_id} =    Get Chain ID
    Should Be True    ${chain_id} > 0

Get Latest Block Number Should Work
    [Documentation]    Test Get Latest Block Number
    ${block_number} =    Get Latest Block Number

Mine Blocks Should Work
    [Documentation]    Test Mine Blocks
    ${previous_block_number} =    Get Latest Block Number
    Mine Blocks    1
    ${current_block_number} =    Get Latest Block Number

Get Balance Should Work
    [Documentation]    Test Get Balance
    ${balance} =    Get Balance    ${ARPA_HOLDER_ADDRESS}
    Should Be True    ${balance} > 0

Set Balance Should Work
    [Documentation]    Test Set Balance
    Set Balance    ${ARPA_HOLDER_ADDRESS}    10
    ${balance} =    Get Balance    ${ARPA_HOLDER_ADDRESS}
    Should Be Equal    ${balance}    10

Wait Block Number Increase
    [Documentation]    Wait for the block number to increase
    [Arguments]    ${number}
    ${original_block_number} =    Get Latest Block Number
    ${block_number} =    Get Latest Block Number
    WHILE    ${original_block_number} + ${number} > ${block_number}
        ${block_number} =    Get Latest Block Number
        Sleep    1s
    END
    Should Be Equal    ${original_block_number} + ${number}    ${block_number}

Source Environment Variables
    [Documentation]    Source environment variables
    ${result} =    Run    source ${ENV_PATH}
    ${result} =    Run    cp ${ENV_PATH} ${CONTRACT_PATH}.env