*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Variables ***
${END_BLOCK}       []

*** Keywords ***

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
    Group Node Number Should Be    0    1
    ${current_randomness} =    Set Variable    1
    ${last_randomness} =    Set Variable    0
    Deploy User Contract

    WHILE    ${current_randomness} != ${last_randomness}
        Request Randomness
        Wait For Process    timeout=20s
        ${log_received_randomness_task} =       All Nodes Have Keyword    received new randomness task
        ...    ${NODE_PROCESS_LIST}    100
        ${last_randomness} =    Set Variable    ${current_randomness}
        ${current_randomness} =    Check Randomness
        Wait For Process    timeout=1m
    END

    Set Global Variable    ${NODE_PROCESS_LIST}    ${EMPTY_LIST}
    Teardown Scenario Testing Environment

Test Log Size
    [Documentation]
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s

    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${node4} =    Stake And Run Node    4
    ${node5} =    Stake And Run Node    5
    ${node6} =    Stake And Run Node    6
    ${node7} =    Stake And Run Node    7
    ${node8} =    Stake And Run Node    8
    ${node9} =    Stake And Run Node    9
    ${node10} =    Stake And Run Node    10


    ${log_group_available} =       All Nodes Have Keyword    Group index:    ${NODE_PROCESS_LIST}
    Wait For Process    timeout=20s
    Group Node Number Should Be    0    10
    ${current_randomness} =    Convert To Integer    0
    Deploy User Contract
    WHILE    ${current_randomness != 60}
        Request Randomness
        Wait For Process    timeout=1s
        ${current_randomness} =    Set Variable    ${current_randomness + 1}
    END
    Teardown Scenario Testing Environment

*** Test Cases ***
Run Long Running Case
    Repeat Keyword    0    Long Running Request Randomness
    Repeat Keyword    0    Test Log Size