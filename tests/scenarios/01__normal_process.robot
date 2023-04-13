*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Test Cases ***
#My Test
    # Log    RUN
    # Deploy Proxy Contract
    # Get Modified CommitdDkg    ${TEST_NODE_ADDRESS}
    # Set Modified CommitdDkg    ${TEST_NODE_ADDRESS}
    # Get Node    ${TEST_NODE_ADDRESS}
    # Get Group    0
    # Get Member    0    0
    # Get Coordinator    0
    # Get Coordinator Instance    0
    # Deploy Coordinator To Test
    # Get Shares
    # Get Justifications
    # Get Participants
    # Get DkgKeys
    # In Phase

# Node Tets
#     List Fixed Tasks    1
#     Shutdown Listener    1    PreGrouping
#     List fixed Tasks    1
#     Start Listener    1    PreGrouping
#     List Fixed Tasks    1
#     #Node Quit    0    unimplemented
#     #Node Register    0
#     #Shutdown Node    0
#     #Activate Node    0    unimplemented
#     Get Node Info    1
#     Get Group Info    1
#     #Post Process Dkg    0

Normal Node Registration
    [Documentation]
    ...    This test case is to test the normal node registration process.
    Set Enviorment And Deploy Contract
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${address} =    Get Address By Index    1
    #Get Node    ${address}
    ${address} =    Get Address By Index    2
    #Get Node    ${address}
    ${address} =    Get Address By Index    3
    #Get Node    ${address}
    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_phase_2} =    All Nodes Have Keyword    Waiting for Phase 2 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    ${log_received_randomness_task} =    All Nodes Have Keyword    Group index:0 epoch:1 is available    ${NODE_PROCESS_LIST}
    ${result} =    Get Group    0
    Group Node Number Should Be    0    3
    ${result} =    Get Coordinator    0
    Deploy User Contract And Request Randomness
    ${log_received_randomness_task} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}
    Sleep    5s
    Mine Blocks    6
    Sleep    10s
    Check Randomness
    Kill Node    ${node1}
    Kill Node    ${node2}
    Kill Node    ${node3}
    Set Global Variable    ${NODE_PROCESS_LIST}    ${EMPTY_LIST}
    #Teardown Scenario Testing Environment