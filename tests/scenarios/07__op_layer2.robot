*** Settings ***
Documentation       Node Registration Scenarios
Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource
Resource            src/op.resource

*** Variables ***


*** Keywords ***

Op test case1
    Set Global Variable    $BLOCK_TIME    1
    Set Op Enviorment And Deploy Contract
    ${node1} =    Stake And Run Node    4
    ${node2} =    Stake And Run Node    5
    ${node3} =    Stake And Run Node    6
    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}    4
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}    4
    ${log_group} =    All Nodes Have Keyword    Group index:0 epoch:1 is available    ${NODE_PROCESS_LIST}    4
    ${log_post_process} =    All Nodes Have keyword    Calling contract transaction post_process_dkg    ${NODE_PROCESS_LIST}    4
    ${result} =    Get Group    0
    Group Node Number Should Be    0    3
    Sleep    2s
    Deploy OP User Contract
    Request Randomness
    ${log_received_randomness_task} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}    4
    Sleep    5s
    Mine Blocks    6
    ${result} =    Have Node Got Keyword    fulfill randomness successfully    ${NODE_PROCESS_LIST}
    Sleep    5s
    Check Randomness
    Teardown Scenario Testing Environment


*** Test Cases ***

Run Test Cases
    Repeat Keyword    1    Op test case1