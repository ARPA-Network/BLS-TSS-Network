*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/local_ethereum.py
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

Happy Path 1
    [Documentation]
    ${node1} =    Add Balance And Run Node    4
    ${node2} =    Add Balance And Run Node    5
    ${node3} =    Add Balance And Run Node    6
    ${address} =    Get Address By Index    4
    Get Node    ${address}
    ${address} =    Get Address By Index    5
    Get Node    ${address}
    ${address} =    Get Address By Index    6
    Get Node    ${address}
    ${log_phase_0} =    Get Keyword From Log    ${node1}    In Phase 0
    ${log_received_randomness_task} =    Get Keyword From Log    ${node1}    Group index:0 epoch:1 is available
    ${result} =    Get Group    0
    Group Node Number Should Be    0    3
    ${result} =    Get Coordinator    0
    Deploy User Contract And Request Randomness
    ${log_received_randomness_task} =    Get Keyword From Log    ${node1}    received new randomness task
    Sleep    10s
    Check Randomness
    Kill Node    ${node1}
    Kill Node    ${node2}
    Kill Node    ${node3}