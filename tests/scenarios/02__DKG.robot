*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Keywords ***

Test Rebalance
    [Documentation]
    ...    1. Given a group of nodes do DKG
    ...    2. When 1, 2, 3, 4, 5 formed a group 0
    ...    3. 6 register as a new node
    ...    4. The rebalance triggered
    ...    5. There will be 2 groups, group 0 and group 1, each group has 3 nodes
    Sleep    3s
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s
    
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${node4} =    Stake And Run Node    4
    ${node5} =    Stake And Run Node    5

    ${result} =    All Nodes Have Keyword    Transaction successful(node_register)    ${NODE_PROCESS_LIST}
    Should Be True    ${result}
    ${get_share} =    All Nodes Have Keyword    Calling contract view get_shares    ${NODE_PROCESS_LIST}
    ${group_result} =    Have Node Got Keyword    Group index:0 epoch:3 is available    ${NODE_PROCESS_LIST}    
    Group Node Number Should Be    0    5

    ${node6} =    Stake And Run Node    6
    ${result} =        Get Keyword From Log    6    Transaction successful(node_register)
    ${get_share} =    All Nodes Have Keyword    Calling contract view get_shares    ${NODE_PROCESS_LIST}
    ${group_result} =    Get Keyword From Log    6    is available

    Group Node Number Should Be    0    3
    Group Node Number Should Be    1    3
    Teardown Scenario Testing Environment

DKG Happy Path1
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

    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${node4} =    Stake And Run Node    4

    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    8
    ${log_phase_2} =   Get Keyword From Log    2    Waiting for Phase 2 to start
    Mine Blocks    9
    ${log_group_available} =   Get Keyword From Log    2    Group index:
    Group Node Number Should Be    0    3
    
    ${slash_event} =    Get Event    ${CONTROLLER_CONTRACT}    NodeSlashed
    Should Be Equal As Strings    ${slash_event['args']['nodeIdAddress']}    ${address4}
    
    Deploy User Contract
    Request Randomness
    Mine Blocks    10
    Have Node Got Keyword    Partial signature sent and accepted by committer    ${NODE_PROCESS_LIST}
    Mine Blocks    10
    Sleep    5s
    Check Randomness
    Teardown Scenario Testing Environment

DKG Happy Path2
    [Documentation]
    ...    1. Given phase 4 times out
    ...    2. When any node calls post_process_dkg
    ...    3. Then reward post_process_dkg caller, have >threshold nodes return correct information
    ...    4. Prepare to enter BLS work
    Set Enviorment And Deploy Contract
    Sleep    3s
    ${address1} =    Get Address By Index    1
    ${address2} =    Get Address By Index    2
    ${address3} =    Get Address By Index    3
    ${address4} =    Get Address By Index    4

    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${node4} =    Stake And Run Node    4

    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    8
    ${log_phase_2} =   Get Keyword From Log    2    Waiting for Phase 2 to start
    Mine Blocks    9
    ${log_group_available} =       All Nodes Have Keyword    Group index:    ${NODE_PROCESS_LIST}
    Group Node Number Should Be    0    4
    ${group} =   Get Group    0
    ${ckeck_result} =    Check Group Status    ${group}
    
    Mine Blocks    20
    Sleep    2s
    ${node_rewards} =    Get Event    ${CONTROLLER_CONTRACT}    NodeRewarded

    ${result} =    Has Equal Value    ${node_rewards['args']['nodeAddress']}    ${address1}    ${address2}    ${address3}    ${address4}
    Should Be True    ${result}

    Deploy User Contract
    Request Randomness
    Mine Blocks    10
    Have Node Got Keyword    Partial signature sent and accepted by committer    ${NODE_PROCESS_LIST}
    Mine Blocks    10
    Sleep    5s
    Check Randomness
    ${group} =   Get Group    0
    Teardown Scenario Testing Environment

DKG Sad Path1
    [Documentation]
    ...    1. Given a group of nodes do DKG
    ...    2. When 1, 2, 3, 4 submit their results
    ...    3. 2 nodes submit r, 2 nodes submit r'.
    ...    4. threshold is 3. r and r' submitter are not pass (2<3)
    ...    5. post_process_dkg triggered, punish node 1 2 3 4
    
    Sleep    3s
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s
    ${address1} =    Get Address By Index    1
    ${address2} =    Get Address By Index    2
    ${address3} =    Get Address By Index    3
    ${address4} =    Get Address By Index    4

    Set Modified Public Key    ${address3}    ${MODIFIRD_PUB_KEY}
    Set Modified Public Key    ${address4}    ${MODIFIRD_PUB_KEY}

    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${node4} =    Stake And Run Node    4

    ${result} =    All Nodes Have Keyword    Transaction successful(node_register)    ${NODE_PROCESS_LIST}

    ${log_phase_1} =    All Nodes Have Keyword    Transaction successful(commit_dkg)    ${NODE_PROCESS_LIST}
    
    Mine Blocks    20
    Sleep    5s
    ${slash_events} =    Get Events    ${CONTROLLER_CONTRACT}    NodeSlashed
    
    ${result} =    Events Should Contain All Value    ${slash_events}    nodeIdAddress    ${address1}    ${address2}    ${address3}    ${address4}
    Should Be True    ${result}
    ${group} =   Get Group    0
    Should Be Equal As Strings    ${group[7]}    False
    Teardown Scenario Testing Environment

*** Test Cases ***

Run DKG Test cases
    Repeat Keyword    1    Test Rebalance
    Repeat Keyword    1    DKG Happy Path1
    Repeat Keyword    1    DKG Happy Path2
    Repeat Keyword    1    DKG Sad Path1
