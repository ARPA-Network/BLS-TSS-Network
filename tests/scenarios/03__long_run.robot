*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Test Cases ***

DKG Happy Path1
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

    ${log_group_available} =   Get Keyword From Log    ${node2}    Group index:
    Group Node Number Should Be    0    3
    ${current_randomness} =    Set Variable    1
    ${last_randomness} =    Set Variable    0

    WHILE    ${current_randomness} != ${last_randomness}
        Deploy User Contract And Request Randomness
        ${log_received_randomness_task} =    Get Keyword From Log   ${node1}    received new randomness task
        Sleep    10s
        ${last_randomness} =    Set Variable    ${current_randomness}
        ${current_randomness} =    Check Randomness
        Sleep    1m
    END

    Kill Node    ${node1}
    Kill Node    ${node2}
    Kill Node    ${node3}
    Set Global Variable    ${NODE_PROCESS_LIST}    ${EMPTY_LIST}
    Teardown Scenario Testing Environment
