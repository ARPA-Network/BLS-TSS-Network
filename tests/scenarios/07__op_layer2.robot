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
    Set Value To Env    IS_STAKE_USER    false
    Set Op Enviorment And Deploy Contract
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    ${log_group} =    All Nodes Have Keyword    Group index:0 epoch:1 is available    ${NODE_PROCESS_LIST}
    ${log_post_process} =    All Nodes Have keyword    Calling contract transaction post_process_dkg    ${NODE_PROCESS_LIST}
    ${result} =    Get Group    0
    Group Node Number Should Be    0    3
    Sleep    10s
    Deploy OP User Contract
    Request Randomness OP
    ${log_received_randomness_task} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}
    ${result} =    Have Node Got Keyword    fulfill randomness successfully    ${NODE_PROCESS_LIST}
    Check Randomness OP
    Teardown OP Environment


*** Test Cases ***

Run Test Cases
    Repeat Keyword    1    Op test case1