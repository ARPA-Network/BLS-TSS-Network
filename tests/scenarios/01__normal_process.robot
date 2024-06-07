*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Keywords ***

Normal Process
    [Documentation]
    ...    This test case is to test the normal node registration process.
    Set Enviorment And Deploy Contract
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${log_phase_1} =    All Nodes Have Keyword    Transaction successful(node_register)    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_group} =    All Nodes Have Keyword    dkg_status transfered from CommitSuccess to WaitForPostProcess    ${NODE_PROCESS_LIST}
    Mine Blocks    20
    Sleep    2s
    ${result} =    Get Group    0
    Group Node Number Should Be    0    3
    Mine Blocks    20
    Sleep    2s
    Deploy User Contract
    Request Randomness
    ${log_received_randomness_task} =    All Nodes Have Keyword    Received randomness task    ${NODE_PROCESS_LIST}
    Sleep    5s
    Mine Blocks    6
    ${result} =    Have Node Got Keyword    Transaction successful(fulfill_randomness)    ${NODE_PROCESS_LIST}
    Sleep    2s
    Check Randomness
    Teardown Scenario Testing Environment

*** Test Cases ***
Run Normal Process
    [Tags]    l1
    Repeat Keyword    1    Normal Process