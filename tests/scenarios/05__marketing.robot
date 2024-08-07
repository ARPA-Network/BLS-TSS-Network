*** Settings ***
Documentation       Marketing Scenarios

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
    Sleep    1s
    ${sub} =    Get Subscription    1
    ${payment} =   Set Variable    ${sub[3]}
    Mine Blocks    10
    ${fullfill_state} =    Wait For State    FulfillmentFinished    task_log    ${NODE_PROCESS_LIST}    ${False}
    Should Be Equal As Strings    ${fullfill_state}    True
    Mine Blocks    10
    Sleep    5s
    ${result} =    Get Latest Event    ${ADAPTER_CONTRACT}    RandomnessRequest
    Payment Should In The Range    ${result['args']['estimatedPayment']}    ${min_value}    ${max_value}

Set Marketing Discount
    [Documentation]
    ...    1. Given a group is ready for randomeness generation
    ...    2. Request randomness and check if the payment is in the correct range
    Set Enviorment And Deploy Contract
    
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${register_state} =    Wait For State    NodeRegistered    group_log    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${register_state}    True

    Mine Blocks    9
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${group_result} =    Wait For State      DKGGroupingAvailable    group_log    ${NODE_PROCESS_LIST}    index=0    epoch=1    
    Should Be Equal As Strings    ${group_result}    True
    Mine Blocks    20
    Sleep    3s

    Deploy User Contract
    Request Randomness
    Mine Blocks    10
    ${log_task_received} =       Wait For State    TaskReceived    task_log    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_task_received}    True
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
    Set Value To Env    USER_PRIVATE_KEY    0x2a871d0798f97d79848a013d4936a73bf4cc922c825d33c1cf7073dff6d409c6
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${register_state} =    Wait For State    NodeRegistered    group_log    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${register_state}    True
    Mine Blocks    9
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${group_state} =    Wait For State    DKGGroupingAvailable    group_log    ${NODE_PROCESS_LIST}    index=0    epoch=1
    Should Be Equal As Strings    ${group_state}    True
    Mine Blocks    20

    Deploy User Contract
    Request Randomness
    Mine Blocks    10
    ${log_task_received} =       Wait For State    TaskReceived    task_log    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_task_received}    True
    Sleep    5s
    
    Execute Script    TestFeeConfig.s.sol:TestFeeConfigScript    ${EMPTY}
    ${original_private_key} =    Get Value From Env    USER_PRIVATE_KEY
    Set Value To Env    USER_PRIVATE_KEY    0xf214f2b2cd398c806f84e317254e0f0b801d0643303237d97a22a48e01628897
    Execute Script    GetRandomNumberScenarioTest.s.sol:GetRandomNumberScenarioTestScript    ${EMPTY}

    Mine Blocks    10
    Sleep    10s
    
    ${contract_address} =    Get Contract Address From File    contracts/broadcast/GetRandomNumberScenarioTest.s.sol/31337/run-latest.json
    ${new_user_contract} =    Get Contract    ${PROXY_OUTPUT}GetRandomNumberExample.sol/GetRandomNumberExample.json    ${contract_address['GetRandomNumberExample']}
    ${result} =    Contract Function Transact    ${new_user_contract}    getRandomNumber
    Mine Blocks    10
    Sleep    10s
    ${log_task_received} =       Wait For State    TaskReceived    task_log    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_task_received}    True

    ${subId1} =    Convert To Integer    1
    ${subId2} =    Convert To Integer    2
    ${adapter_address} =    Get Value From Env    ADAPTER_ADDRESS
    
    ${result} =    Cast Send    ${adapter_address}    "setReferral(uint64, uint64)" ${subId1} ${subId2}    ${original_private_key}    ${EMPTY}
    
    Request Randomenes And Check Payment    0    100000000000000000
    Request Randomenes And Check Payment    0    100000000000000000

    Contract Function Transact    ${new_user_contract}    getRandomNumber
    ${sub} =    Get Subscription    2
    ${payment} =   Set Variable    ${sub[3]}
    Payment Should In The Range    ${payment}    0    100000000000000000
    Mine Blocks    10
    All Nodes Have Keyword    Received randomness task    ${NODE_PROCESS_LIST}

    Contract Function Transact    ${new_user_contract}    getRandomNumber
    ${sub} =    Get Subscription    2
    ${payment} =   Set Variable    ${sub[3]}
    Payment Should In The Range    ${payment}    0    100000000000000000
    Mine Blocks    10
    ${log_task_received} =       Wait For State    TaskReceived    task_log    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_task_received}    True

    
    Teardown Scenario Testing Environment

*** Test Cases ***

Run Test Cases
    [Tags]    l1
    Repeat Keyword    1    Set Marketing Discount
    Repeat Keyword    1    Test Referral