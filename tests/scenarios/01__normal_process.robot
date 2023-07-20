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
    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_group} =    All Nodes Have Keyword    Group index:0 epoch:1 is available    ${NODE_PROCESS_LIST}
    ${result} =    Get Group    0
    Group Node Number Should Be    0    3
    ${result} =    Get Coordinator    0
    Deploy User Contract
    Request Randomness
    ${log_received_randomness_task} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}
    Sleep    5s
    Mine Blocks    6
    ${result} =    Have Node Got Keyword    fulfill randomness successfully    ${NODE_PROCESS_LIST}
    Sleep    5s
    Check Randomness
    Teardown Scenario Testing Environment

*** Test Cases ***
Run Normal Process
    Repeat Keyword    1    Normal Process