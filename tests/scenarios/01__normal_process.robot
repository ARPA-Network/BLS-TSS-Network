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
    Sleep    2s
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${log_phase_1} =    All Nodes Have Keyword    Transaction successful(node_register)    ${NODE_PROCESS_LIST}    500
    Should Be Equal As Strings    ${log_phase_1}    True
    Mine Blocks    9
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_phase_2}    True
    Mine Blocks    9
    ${log_group} =    All Nodes Have Keyword    dkg_status transfered from CommitSuccess to WaitForPostProcess    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${log_group}    True
    Mine Blocks    20
    Sleep    2s
    ${result} =    Get Group    0
    Group Node Number Should Be    0    3
    Mine Blocks    25
    Sleep    10s
    Deploy User Contract
    Request Randomness
    Mine Blocks    10
    ${result} =    Have Node Got Keyword    Transaction successful(fulfill_randomness)    ${NODE_PROCESS_LIST}
    Sleep    5s
    Check Randomness
    Teardown Scenario Testing Environment

*** Test Cases ***
Run Normal Process
    [Tags]    l1
    Repeat Keyword    1    Normal Process