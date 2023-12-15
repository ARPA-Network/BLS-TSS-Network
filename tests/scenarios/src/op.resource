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
${OP_DIR}        ../optimism

*** Keywords ***

Setup OP Devnet
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
    ${relay_config} =    Create Relay List    ws://127.0.0.1:9546    901    ws://127.0.0.1:9646    8453
    Create Node Config    ${CONTRACT_ADDRESSES['Controller']}    ${CONTRACT_ADDRESSES['ERC1967Proxy']}    ${CONTRACT_ADDRESSES['ControllerRelayer']}    900    ${relay_config}

Set Op Enviormnet
    ${result} =    Exec Script    OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript    http://localhost:9645
    ${contract_addresses} =    Get Contract Address From File    contracts/broadcast/OPControllerOracleLocalTest.s.sol/8453/run-latest.json
    Set Environment Variable    BASE_ARPA_ADDRESS    ${contract_addresses['Arpa']}
    Set Environment Variable    BASE_CONTROLLER_ORACLE_ADDRESS    ${contract_addresses['ControllerOracle']}
    Set Environment Variable    BASE_ADAPTER_ADDRESS    ${contract_addresses['ERC1967Proxy']}
    ${result} =    Exec Script    OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript    http://localhost:9545
    ${contract_addresses} =    Get Contract Address From File    contracts/broadcast/OPControllerOracleLocalTest.s.sol/901/run-latest.json
    Set Environment Variable    OP_ARPA_ADDRESS    ${contract_addresses['Arpa']}
    Set Environment Variable    OP_CONTROLLER_ORACLE_ADDRESS    ${contract_addresses['ControllerOracle']}
    Set Environment Variable    OP_ADAPTER_ADDRESS    ${contract_addresses['ERC1967Proxy']}
 
    ${result} =    Exec Script    ControllerLocalTestScenario.s.sol:ControllerLocalTestScript
    
    ${contract_addresses} =    Get Contract Address From File    contracts/broadcast/ControllerLocalTestScenario.s.sol/900/run-latest.json
    Set Global Variable    $CONTRACT_ADDRESSES    ${contract_addresses}
    Set Environment Variable    ARPA_ADDRESS    ${contract_addresses['Arpa']}
    Set Environment Variable    CONTROLLER_ADDRESS    ${contract_addresses['Controller']}
    Set Environment Variable    STAKING_ADDRESS    ${contract_addresses['Staking']}
    Set Environment Variable   ADAPTER_ADDRESS    ${contract_addresses['ERC1967Proxy']}
    Set Environment Variable    OP_CHAIN_MESSENGER_ADDRESS    ${contract_addresses['OPChainMessenger']}
    ${result} =    Exec Script    OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript    http://localhost:9545
    ${result} =    Exec Script    OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript    http://localhost:9645
    ${result} =    Exec Script    InitStakingLocalTest.s.sol:InitStakingLocalTestScript
    ${result} =    Exec Script    StakeOperatorScenarioTest.s.sol:StakeOperatorScenarioTestScript

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
    [Arguments]    ${url}    ${chain_id}
    ${contract_output} =    Exec Script    OPGetRandomNumberLocalTest.s.sol:OPGetRandomNumberLocalTestScript    ${url}
    ${contract_address} =    Get Contract Address From File    contracts/broadcast/OPGetRandomNumberLocalTest.s.sol/${chain_id}/run-latest.json
    Set Global Variable    $USER_CONTRACT_ADDRESS    ${contract_address['GetRandomNumberExample']}

Teardown OP Environment
    [Documentation]    Tears down the OP scenario testing environment
    ${result} =     Run Process     make    devnet-clean    cwd=${OP_DIR}
    Kill Previous Node    20
    Set Global Variable    $NODE_PROCESS_LIST    ${EMPTY_LIST}
    Set Global Variable    $BLOCK_TIME    0
    Set Global Variable    $USER_CONTRACT_ADDRESS    NULL
    Clear Database
    Terminate All Processes    kill=true

Request Randomness OP
    [Documentation]    Cast request randomness
    [Arguments]    ${url}
    ${key} =       Get Value From Env    USER_PRIVATE_KEY
    ${result} =    Cast Send    ${USER_CONTRACT_ADDRESS}    "requestRandomness()"  ${key}    ${url}

Check Randomness OP
    [Documentation]    Check the randomness
    [Arguments]    ${url}
    ${last_output} =    Run    cast call ${USER_CONTRACT_ADDRESS} "lastRandomnessResult()(uint256)" --rpc-url=${url}
    Log    ${last_output}
    RETURN    ${last_output}