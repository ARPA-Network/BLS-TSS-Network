*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/local_ethereum.py
Resource            src/common.resource
Resource            src/contract.resource

*** Test Cases ***
My Test
    Log    RUN
    Deploy Proxy Contract
    Get Modified CommitdDkg    ${TEST_NODE_ADDRESS}
    Set Modified CommitdDkg    ${TEST_NODE_ADDRESS}
    Get Node    ${TEST_NODE_ADDRESS}
    Get Group    0
    Node In Members    0    ${TEST_NODE_ADDRESS}
    Partial Key Registered    0    ${TEST_NODE_ADDRESS}
    #Get Member    0    0
    Get Coordinator    0
    Get Coordinator Instance    0
    Deploy Coordinator To Test
    Get Shares
    Get Justifications
    Get Participants
    Get DkgKeys
    In Phase