*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Keywords ***

Payment Should In The Range
    [Arguments]    ${payment}    ${min_value}    ${max_value}
    ${payment} =    Convert To Integer    ${payment}
    ${min_value} =    Convert To Integer    ${min_value}
    ${max_value} =    Convert To Integer    ${max_value}
    Should Be True    ${payment} >= ${min_value}
    Should Be True    ${payment} <= ${max_value}

Request Randomenes And Check Payment
    [Arguments]    ${min_value}    ${max_value}
    ${result} =    Request Randomness
    ${sub} =    Get Subscription    1
    ${payment} =   Set Variable    ${sub[3]}
    Mine Blocks    10
    All Nodes Have Keyword    Partial signature sent and accepted by committer    ${NODE_PROCESS_LIST}
    Mine Blocks    10
    Sleep    5s
    ${result} =    Get Event    ${ADAPTER_CONTRACT}    RandomnessRequest
    Payment Should In The Range    ${payment}    ${min_value}    ${max_value}

Set Marketing Discount
    [Documentation]
    ...    1. Given a group is ready for randomeness generation
    ...    2. Request randomness and check if the payment is in the correct range
    Set Enviorment And Deploy Contract
    
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_group} =    All Nodes Have Keyword    Group index:0 epoch:1 is available    ${NODE_PROCESS_LIST}

    Deploy User Contract
    Request Randomness
    Mine Blocks    10
    All Nodes Have Keyword    Partial signature sent and accepted by committer    ${NODE_PROCESS_LIST}
    Mine Blocks    10
    Sleep    5s

    Request Randomenes And Check Payment    50000000000000000    100000000000000000

    Request Randomenes And Check Payment    100000000000000000    200000000000000000

    Request Randomenes And Check Payment    150000000000000000    200000000000000000

    Request Randomenes And Check Payment    200000000000000000    300000000000000000

    # Set timestamp 1 day later
    Set Timestamp Interval    86400
    Mine Blocks    1

    Request Randomenes And Check Payment    50000000000000000    100000000000000000
    
    Teardown Scenario Testing Environment

Test Referral
    [Documentation]
    ...    1. Given a group is ready for randomeness generation
    ...    2. Referral a new user and check if the payment is only the gas fee
    Set Enviorment And Deploy Contract
    
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_group} =    All Nodes Have Keyword    Group index:0 epoch:1 is available    ${NODE_PROCESS_LIST}

    Deploy User Contract
    Request Randomness
    Mine Blocks    10
    All Nodes Have Keyword    Partial signature sent and accepted by committer    ${NODE_PROCESS_LIST}
    Sleep    10s
    
    Exec Script    TestFeeConfig.s.sol:TestFeeConfigScript
    ${original_private_key} =    Get Value From Env    USER_PRIVATE_KEY
    Set Value To Env    USER_PRIVATE_KEY    0xf214f2b2cd398c806f84e317254e0f0b801d0643303237d97a22a48e01628897
    Exec Script    GetRandomNumberScenarioTest.s.sol:GetRandomNumberScenarioTestScript

    Mine Blocks    10
    Sleep    10s
    
    ${contract_address} =    Get Contract Address From File    contracts/broadcast/GetRandomNumberScenarioTest.s.sol/31337/run-latest.json
    ${new_user_contract} =    Get Contract    ${PROXY_OUTPUT}GetRandomNumberExample.sol/GetRandomNumberExample.json    ${contract_address[0]}
    ${result} =    Contract Function Transact    ${new_user_contract}    getRandomNumber
    Mine Blocks    10
    Sleep    10s
    ${result} =    All Nodes Have Keyword    Partial signature sent and accepted by committer    ${NODE_PROCESS_LIST}
    ${subId1} =    Convert To Integer    1
    ${subId2} =    Convert To Integer    2
    ${adapter_address} =    Get Value From Env    ADAPTER_ADDRESS
    
    ${result} =    Run    cast send ${adapter_address} "setReferral(uint64, uint64)" ${subId1} ${subId2} --private-key ${original_private_key}
    
    Request Randomenes And Check Payment    0    100000000000000000
    Request Randomenes And Check Payment    0    100000000000000000

    Contract Function Transact    ${new_user_contract}    getRandomNumber
    ${sub} =    Get Subscription    2
    ${payment} =   Set Variable    ${sub[3]}
    Payment Should In The Range    ${payment}    0    100000000000000000
    Mine Blocks    10
    All Nodes Have Keyword    Partial signature sent and accepted by committer    ${NODE_PROCESS_LIST}

    Contract Function Transact    ${new_user_contract}    getRandomNumber
    ${sub} =    Get Subscription    2
    ${payment} =   Set Variable    ${sub[3]}
    Payment Should In The Range    ${payment}    0    100000000000000000
    Mine Blocks    10
    All Nodes Have Keyword    Partial signature sent and accepted by committer    ${NODE_PROCESS_LIST}

    
    Teardown Scenario Testing Environment

*** Test Cases ***

Run Test Cases
    Repeat Keyword    1    Set Marketing Discount
    Repeat Keyword    1    Test Referral