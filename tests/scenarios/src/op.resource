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
    IF    ${LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE != ''}
        Terminate Process    ${LOCAL_ETHEREUM_CHAIN_PROCESS_HANDLE}
    END

    ${result} =     Run Process     make    devnet-clean    cwd=${OP_DIR}
    ${process_handle} =     Start Process     make    devnet-up-deploy    cwd=${OP_DIR}    stdout=op.log
    Wait For Keyword From Log    ${OP_DIR}/op.log    Devnet ready    max_retry_time=1500
    ${result} =     Run Process     docker    container    ls
    log    ${result.stdout}
    
Set Op Enviorment And Deploy Contract
    [Documentation]    Set enviorment and deploy proxy contract
    Clear Log
    Kill Previous Node    20
    Clear Database
    Set Global Variable    $NODE_PROCESS_LIST    ${EMPTY_LIST}
    Setup OP Devnet
    Source Environment Variables
    ${result} =    Exec Script    ControllerLocalTest.s.sol:ControllerLocalTestScript
    ${contract_addresses} =    Get Contract Address From File    contracts/broadcast/ControllerLocalTest.s.sol/900/run-latest.json
    Set Global Variable    $CONTRACT_ADDRESSES    ${contract_addresses}
    ${result} =    Exec Script    StakeOperatorScenarioTest.s.sol:StakeOperatorScenarioTestScript
    Bound OP Contract ABI
    Set ARPA Address
    Set Op Enviormnet
    Parse Node
    Create Node Config    ${CONTRACT_ADDRESSES['Controller']}    ${CONTRACT_ADDRESSES['ERC1967Proxy']}    ${CONTRACT_ADDRESSES['ControllerRelayer']}    900

Set Op Enviormnet
    ${result} =    Exec Script    OPControllerOracleLocalTest.s.sol:OPControllerOracleLocalTestScript    http://localhost:9545
    ${contract_addresses} =    Get Contract Address From File    contracts/broadcast/OPControllerOracleLocalTest.s.sol/901/run-latest.json
    Set Environment Variable    OP_ARPA_ADDRESS    ${contract_addresses['Arpa']}
    Set Environment Variable    OP_CONTROLLER_ORACLE_ADDRESS    ${contract_addresses['ControllerOracle']}
    Set Environment Variable    OP_ADAPTER_ADDRESS    ${contract_addresses['ERC1967Proxy']}
    ${result} =    Exec Script    OPControllerOracleInitializationLocalTest.s.sol:OPControllerOracleInitializationLocalTestScript    http://localhost:9545

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
    Set Global Variable    $USER_CONTRACT_ADDRESS    ${contract_address['default']}  
    ${user_contract} =    Get Contract    ${PROXY_OUTPUT}GetRandomNumberExample.sol/GetRandomNumberExample.json    ${user_contract_address}
    Set Global Variable    $USER_CONTRACT   ${user_contract}