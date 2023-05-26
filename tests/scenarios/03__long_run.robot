*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Test Cases ***

Long Running Request Randomness
    [Documentation]
    ...    1. Given a group is ready for randomeness generation
    ...    2. Request randomness ervery minute
    ...    3. Check randomness is generated
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s

    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3

    ${log_group_available} =       All Nodes Have Keyword    Group index:    ${NODE_PROCESS_LIST}
    Wait For Process    timeout=20s
    Group Node Number Should Be    0    3
    ${current_randomness} =    Set Variable    1
    ${last_randomness} =    Set Variable    0

    WHILE    ${current_randomness} != ${last_randomness}
        Deploy User Contract And Request Randomness
        Wait For Process    timeout=20s
        ${log_received_randomness_task} =       All Nodes Have Keyword    received new randomness task
        ...    ${NODE_PROCESS_LIST}    100
        ${last_randomness} =    Set Variable    ${current_randomness}
        ${current_randomness} =    Check Randomness
        Wait For Process    timeout=1m
    END

    Set Global Variable    ${NODE_PROCESS_LIST}    ${EMPTY_LIST}
    Teardown Scenario Testing Environment

*** Keywords ***
Run Long Running Case
    #Long Running Request Randomness