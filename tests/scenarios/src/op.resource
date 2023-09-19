*** Settings ***
Documentation       This resource file sets the constants and variables
...                 that applies to all test suites and test cases

Library             Process
Library             String
Library             OperatingSystem
Library             environment/node.py
Library             environment/util.py
Library             environment/log.py
Resource            common.resource

*** Variables ***
${OP_DIR}        ../optimism/

*** Keywords ***

Setup OP Devnet
    Clear One Log    ${OP_DIR}/op.log
    Kill Process By Name    anvil
    ${result} =     Run Process     make    devnet-clean    cwd=${OP_DIR}
    ${process_handle} =     Start Process     make    devnet-up-deploy    cwd=${OP_DIR}    stdout=op.log
    Wait For Keyword From Log    ${OP_DIR}/op.log    Devnet ready    max_retry_time=1500
    ${l1_cross_domain_messager_address} =    Get Address From File    ${OP_DIR}/op.log    L1CrossDomainMessengerProxy
    Set Environment Variable    OP_L1_CROSS_DOMAIN_MESSENGER_ADDRESS    ${l1_cross_domain_messager_address}
    ${result} =     Run Process     docker    container    ls
    log    ${result.stdout}
    
Set Op Enviorment And Deploy Contract
    [Documentation]    Set enviorment and deploy proxy contract
    Clear Log
    Kill Previous Node    20
    Clear Database
    Set Global Variable    $NODE_PROCESS_LIST    ${EMPTY_LIST}
    Setup OP Devnet

    Set Op Enviormnet
    Bound OP Contract ABI
    Set ARPA Address
    Parse Node
    Create Node Config    ${CONTRACT_ADDRESSES['Controller']}    ${CONTRACT_ADDRESSES['ERC1967Proxy']}    ${CONTRACT_ADDRESSES['ControllerRelayer']}    900

Set Op Enviormnet
    ${result} =    Exec Script    OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript    http://localhost:9545
    ${contract_addresses} =    Get Contract Address From File    contracts/broadcast/OPControllerOracleLocalTest.s.sol/901/run-latest.json
    Set Environment Variable    OP_ARPA_ADDRESS    ${contract_addresses['Arpa']}
    Set Environment Variable    OP_CONTROLLER_ORACLE_ADDRESS    ${contract_addresses['ControllerOracle']}
    Set Environment Variable    OP_ADAPTER_ADDRESS    ${contract_addresses['ERC1967Proxy']}
    ${result} =    Exec Script    ControllerLocalTest.s.sol:ControllerLocalTestScript
    
    ${contract_addresses} =    Get Contract Address From File    contracts/broadcast/ControllerLocalTest.s.sol/900/run-latest.json
    Set Global Variable    $CONTRACT_ADDRESSES    ${contract_addresses}
    Set Environment Variable    ARPA_ADDRESS    ${contract_addresses['Arpa']}
    Set Environment Variable    CONTROLLER_ADDRESS    ${contract_addresses['Controller']}
    Set Environment Variable    STAKING_ADDRESS    ${contract_addresses['Staking']}
    Set Environment Variable   ADAPTER_ADDRESS    ${contract_addresses['ERC1967Proxy']}
    Set Environment Variable    OP_CHAIN_MESSENGER_ADDRESS    ${contract_addresses['OPChainMessenger']}
    
    ${result} =    Exec Script    OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript    http://localhost:9545
    ${result} =    Exec Script    InitStakingLocalTest.s.sol:InitStakingLocalTestScript
    ${result} =    Exec Script    StakeNodeLocalTest.s.sol:StakeNodeLocalTestScript

Bound OP Contract ABI
    [Documentation]    Bound contract ABI to handle contract function call
    ${proxy_address} =    Set Variable    ${CONTRACT_ADDRESSES['Controller']}
    ${proxy_address} =    To Checksum Address    ${proxy_address}
    Set Global Variable    $PROXY_CONTRACT_ADDRESS    ${proxy_address}
    ${proxy_contract} =    Get Contract    ${PROXY_OUTPUT}Controller.sol/Controller.json    ${PROXY_CONTRACT_ADDRESS}
    Set Global Variable    $PROXY_CONTRACT   ${proxy_contract}
    ${controller_contract} =    Get Contract    ${PROXY_OUTPUT}Controller.sol/Controller.json    ${PROXY_CONTRACT_ADDRESS}
    Set Global Variable    $CONTROLLER_CONTRACT   ${controller_contract}
    ${stake_address} =    Get Value From Env    STAKING_ADDRESS
    ${stake_address} =    To Checksum Address    ${stake_address}
    ${stake_contract} =    Get Contract    ${PROXY_OUTPUT}Staking.sol/Staking.json    ${stake_address}
    Set Global Variable    $STAKE_CONTRACT   ${stake_contract}
    ${adapter_address} =    Get Value From Env    ADAPTER_ADDRESS
    ${adapter_address} =    To Checksum Address    ${adapter_address}
    ${adapter_contract} =    Get Contract    ${PROXY_OUTPUT}Adapter.sol/Adapter.json    ${adapter_address}
    Set Global Variable    $ADAPTER_CONTRACT   ${adapter_contract}

Deploy OP User Contract
    [Documentation]    Deploy user request contract
    ${contract_output} =    Exec Script    OPGetRandomNumberLocalTest.s.sol:OPGetRandomNumberLocalTestScript    http://localhost:9545
    ${contract_address} =    Get Contract Address From File    contracts/broadcast/OPGetRandomNumberLocalTest.s.sol/901/run-latest.json
    Set Global Variable    $USER_CONTRACT_ADDRESS    ${contract_address['GetRandomNumberExample']}

Teardown OP Environment
    [Documentation]    Tears down the OP scenario testing environment
    ${result} =     Run Process     make    devnet-clean    cwd=${OP_DIR}
    ${list_len} =   Get Length    ${NODE_PROCESS_LIST}
    Kill Previous Node    ${list_len}
    Set Global Variable    $NODE_PROCESS_LIST    ${EMPTY_LIST}
    Set Global Variable    $BLOCK_TIME    0
    Set Global Variable    $USER_CONTRACT_ADDRESS    NULL
    Clear Database
    Terminate All Processes

Request Randomness OP
    [Documentation]    Cast request randomness
    ${key} =    Get Value From Env    USER_PRIVATE_KEY
    ${result} =    Run    cast send ${USER_CONTRACT_ADDRESS} "requestRandomness()" --private-key=${key} --rpc-url=http://127.0.0.1:9545 --gas-limit=100000

Check Randomness OP
    [Documentation]    Check the randomness
    ${last_output} =    Run    cast call ${USER_CONTRACT_ADDRESS} "lastRandomnessResult()(uint256)" --rpc-url=http://127.0.0.1:9545
    Log    ${last_output}
    RETURN    ${last_output}