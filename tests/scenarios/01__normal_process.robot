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
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Mine Blocks    20
    Sleep    3s
    Mine Blocks    20
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${dkg_state} =     Wait For State    DKGKeyGenerated    group_log    ${NODE_PROCESS_LIST}
    Should Be Equal As Strings    ${dkg_state}    True
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}    300
    Should Be Equal As Strings    ${log_phase_2}    True
    ${group_state} =    Wait For State    DKGGroupingAvailable    group_log    ${NODE_PROCESS_LIST}    index=0    epoch=1
    Should Be Equal As Strings    ${group_state}    True
    ${result} =    Get Group    0
    Group Node Number Should Be    0    3
    Mine Blocks    20
    Sleep    5s
    Deploy User Contract
    Request Randomness
    ${fullfill_state} =    Wait For State    FulfillmentFinished    task_log    ${NODE_PROCESS_LIST}    ${False}
    Check Randomness
    Teardown Scenario Testing Environment

*** Test Cases ***
Run Normal Process
    [Tags]    l1
    Repeat Keyword    1    Normal Process