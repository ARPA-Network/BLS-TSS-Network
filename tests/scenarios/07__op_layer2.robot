*** Settings ***
Documentation       Node Registration Scenarios
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
    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_phase_1}    True
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_phase_2}    True
    ${log_group} =    All Nodes Have Keyword    Group index:0 epoch:1 is available    ${NODE_PROCESS_LIST}
    ${log_post_process} =    All Nodes Have keyword    Calling contract transaction post_process_dkg    ${NODE_PROCESS_LIST}
    ${result} =    Get Group    0
    Group Node Number Should Be    0    3
    ${log_post_process} =    Have Node Got Keyword    Transaction successful(relay_group)    ${NODE_PROCESS_LIST}
    Sleep    5s
    Deploy OP User Contract    http://localhost:9645    8453
    Request Randomness OP    http://localhost:9645
    ${log_received_randomness_task} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}
    ${result} =    Have Node Got Keyword    fulfill randomness successfully    ${NODE_PROCESS_LIST}
    Check Randomness OP    http://localhost:9645

    Deploy OP User Contract    http://localhost:9545    901
    Request Randomness OP    http://localhost:9545
    ${log_received_randomness_task} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}
    ${result} =    Have Node Got Keyword    fulfill randomness successfully    ${NODE_PROCESS_LIST}
    Check Randomness OP    http://localhost:9545
    #Teardown OP Environment

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
    ${log_received_randomness_task} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}
    ${result} =    Have Node Got Keyword    fulfill randomness successfully    ${NODE_PROCESS_LIST}
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
    ${log_received_randomness_task} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_received_randomness_task}    True
    ${result} =    Have Node Got Keyword    fulfill randomness successfully    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${result}    True
    Check Randomness OP    http://localhost:10045
    Teardown OP Environment

*** Test Cases ***

Run Test Cases
    [Tags]    l2
    Repeat Keyword    1    L2 Normal Process
    Repeat Keyword    1    L2 Connect Retry