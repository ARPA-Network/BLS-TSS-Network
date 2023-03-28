*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Test Cases ***

DKG Happy Path
    [Documentation]
    ...    1. Given a group of nodes do DKG
    ...    2. When 1, 2, 3, 4 submit their results
    ...    3. 4 is disqualified from 123, results submitted from 123 will indicate as such
    ...    4. DKG phase time out(Anvil mines blocks)
    ...    5. Any node calls post_process_dkg
    ...    6. DKG is successful, 4 is slashed
    Set Enviorment And Deploy Contract
    Sleep    3s
    ${address1} =    Get Address By Index    1
    ${address2} =    Get Address By Index    2
    ${address3} =    Get Address By Index    3
    ${address4} =    Get Address By Index    4

    ${address4} =    To Checksum Address    ${address4}
    ${disqulified_list} =    Make List    ${address4}
    Set Modified Disqualified Nodes    ${address1}    ${disqulified_list}
    Set Modified Disqualified Nodes    ${address2}    ${disqulified_list}
    Set Modified Disqualified Nodes    ${address3}    ${disqulified_list}
    Set Modified Public Key    ${address4}    ${MODIFIRD_PUB_KEY}

    ${node1} =    Add Balance And Run Node    1
    ${node2} =    Add Balance And Run Node    2
    ${node3} =    Add Balance And Run Node    3
    ${node4} =    Add Balance And Run Node    4

    ${disqulified_node_staking_before_slash} =    Get Node Staking    ${address4}

    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    8
    ${log_phase_2} =   Get Keyword From Log    ${node2}    Waiting for Phase 2 to start
    Mine Blocks    9
    ${log_group_available} =   Get Keyword From Log    ${node2}    Group index:
    Group Node Number Should Be    0    3
    ${result} =    Get Coordinator    1

    ${disqulified_node_staking_after_slash} =    Get Node Staking    ${address4}
    Should Be Equal As Integers    ${disqulified_node_staking_before_slash}    ${disqulified_node_staking_after_slash + 1000}

    Deploy User Contract And Request Randomness
    ${log_received_randomness_task} =    Get Keyword From Log   ${node1}    received new randomness task
    Sleep    5s
    Mine Blocks    6
    Sleep    5s
    Check Randomness
    Kill Node    ${node1}
    Kill Node    ${node2}
    Kill Node    ${node3}
    Kill Node    ${node4}
    Set Global Variable    ${NODE_PROCESS_LIST}    ${EMPTY_LIST}
    