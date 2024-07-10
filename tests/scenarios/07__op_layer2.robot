*** Settings ***
Documentation       Layer2 Scenarios
Library             src/environment/contract.py
Library             src/environment/log.py
Library             src/environment/node.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource
Resource            src/op.resource

*** Variables ***


*** Keywords ***

L2 Normal Process
    Set Global Variable    $BLOCK_TIME    1
    Set Value To Env    IS_ADD_OPERATOR    false
    Set Op Enviorment And Deploy Contract
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${node4} =    Stake And Run Node    4
    ${node5} =    Stake And Run Node    5
    ${group_state} =    Wait For State One Node    DKGGroupingAvailable    group_log    5
    Should Be Equal As Strings    ${group_state}    True
    ${node6} =    Stake And Run Node    6
    ${node7} =    Stake And Run Node    7
    ${node8} =    Stake And Run Node    8
    ${node9} =    Stake And Run Node    9
    ${group_state} =    Wait For State One Node    DKGGroupingAvailable    group_log    9
    Should Be Equal As Strings    ${group_state}    True
    ${node10} =    Stake And Run Node    10
    ${node11} =    Stake And Run Node    11
    ${node12} =    Stake And Run Node    12
    ${node13} =    Stake And Run Node    13
    ${group_state} =    Wait For State One Node    DKGGroupingAvailable    group_log    13
    Should Be Equal As Strings    ${group_state}    True
    ${node14} =    Stake And Run Node    14
    ${node15} =    Stake And Run Node    15
    ${node16} =    Stake And Run Node    16
    ${node17} =    Stake And Run Node    17
    ${node18} =    Stake And Run Node    18
    ${node19} =    Stake And Run Node    19
    #${node20} =    Stake And Run Node    20
    
    ${group_state} =    Wait For State One Node    DKGGroupingAvailable    group_log    19
    Should Be Equal As Strings    ${group_state}    True

    Sleep    100s

    ${result} =    Get Group    0
    ${result} =    Get Group    1
    ${result} =    Get Group    2
    ${result} =    Get Group    3
    ${log_relay} =    Have Node Got Keyword    Transaction successful(relay_group)    ${NODE_PROCESS_LIST}
    # Deploy OP User Contract    http://localhost:9645    8453
    # Request Randomness OP    http://localhost:9645
    # ${log_received_randomness_task} =    All Nodes Have Keyword    Received randomness task    ${NODE_PROCESS_LIST}
    # ${result} =    Have Node Got Keyword    Transaction successful(fulfill_randomness)    ${NODE_PROCESS_LIST}
    # Check Randomness OP    http://localhost:9645

    Deploy OP User Contract    http://localhost:9545    901
    Request Randomness OP    http://localhost:9545
    ${log_task_received} =       Wait For State    TaskReceived    task_log    ${NODE_PROCESS_LIST}    ${False}
    Should Be Equal As Strings    ${log_task_received}    True

    ${fullfill_state} =    Wait For State    FulfillmentFinished    task_log    ${NODE_PROCESS_LIST}    ${False}
    Should Be Equal As Strings    ${fullfill_state}    True

    Check Randomness OP    http://localhost:9545
    Teardown OP Environment

L2 Connect Retry
    [Documentation]
    ...    This case must run after [L2 Normal Process] case.
    ...    Test the retry strategy of the node.
    Kill Previous Node    20
    ${L2_process} =    Start Process    anvil    -p    10045    -f    http://localhost:9545    --block-time    2
    ${relay_config} =    Create Relay List    ws://127.0.0.1:10045    901
    Create Node Config    ${CONTRACT_ADDRESSES['Controller']}    ${CONTRACT_ADDRESSES['ERC1967Proxy']}    ${CONTRACT_ADDRESSES['ControllerRelayer']}    900    ${relay_config}
    Start Node    1
    Start Node    2
    Start Node    3
    All Nodes Have Keyword    Config    ${NODE_PROCESS_LIST}
    Sleep    10s
    Deploy OP User Contract    http://localhost:10045    901
    ${log_received_randomness_task} =    All Nodes Have Keyword    Received randomness task    ${NODE_PROCESS_LIST}
    ${result} =    Have Node Got Keyword    Transaction successful(fulfill_randomness)    ${NODE_PROCESS_LIST}
    Check Randomness OP    http://localhost:10045
    Terminate Process    ${L2_process}    kill=true
    All Nodes Have Keyword    Error during reconnection    ${NODE_PROCESS_LIST}
    All Nodes Have Keyword    Handle interruption for NewRandomnessTaskListener    ${NODE_PROCESS_LIST}
    ${L2_process} =    Start Process    anvil    -p    10045    -f    http://localhost:9545    --block-time    2
    Process Should Be Running    ${L2_process}
    Sleep    5s
    Deploy OP User Contract    http://localhost:10045    901
    Deploy OP User Contract    http://localhost:10045    901
    Request Randomness OP    http://localhost:10045
    ${log_received_randomness_task} =    All Nodes Have Keyword    Received randomness task    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_received_randomness_task}    True
    ${result} =    Have Node Got Keyword    Transaction successful(fulfill_randomness)    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${result}    True
    Check Randomness OP    http://localhost:10045
    Teardown OP Environment

*** Test Cases ***

Run Test Cases
    [Tags]    l2
    Repeat Keyword    1    L2 Normal Process
    #Repeat Keyword    1    L2 Connect Retry