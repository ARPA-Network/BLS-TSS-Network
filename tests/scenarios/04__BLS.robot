*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Test Cases ***

BLS Happy Path1
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
    ${node4} =    Stake And Run Node    4
    ${node5} =    Stake And Run Node    5


    ${log_group_available} =       All Nodes Have Keyword    Group index:    ${NODE_PROCESS_LIST}
    Wait For Process    timeout=20s

    ${node6} =    Stake And Run Node    6
    ${node6_in_group} =    Get Keyword From Log    6    Group index:
    
    ${current_randomness} =    Set Variable    1
    ${last_randomness} =    Set Variable    0

    WHILE    ${current_randomness} != ${last_randomness}
        Deploy User Contract And Request Randomness
        Wait For Process    timeout=30s
        ${last_randomness} =    Set Variable    ${current_randomness}
        ${current_randomness} =    Check Randomness
        Wait For Process    timeout=1m
    END

    Set Global Variable    ${NODE_PROCESS_LIST}    ${EMPTY_LIST}
    #Teardown Scenario Testing Environment
