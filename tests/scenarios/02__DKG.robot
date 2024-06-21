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

    ${group_result} =    All Nodes Have Keyword    dkg_status transfered from CommitSuccess to WaitForPostProcess    ${NODE_PROCESS_LIST}    
    Group Node Number Should Be    0    5

    ${node6} =    Stake And Run Node    6
    ${result} =        Get Keyword From Node Log    6    Transaction successful(node_register)

    ${group_result} =    Get Keyword From Node Log    6    is available

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
    ${log_phase_2} =   Get Keyword From Node Log    2    Waiting for Phase 2 to start
    Mine Blocks    9
    ${log_group_available} =   Get Keyword From Node Log    2    dkg_status transfered from CommitSuccess to WaitForPostProcess:
    Mine Blocks    20
    Sleep    2s
    Group Node Number Should Be    0    3
    
    ${slash_event} =    Get Event    ${NODE_REGISTRY_CONTRACT}    NodeSlashed
    Should Be Equal As Strings    ${slash_event['args']['nodeIdAddress']}    ${address4}
    Mine Blocks    20
    Sleep    10s
    Deploy User Contract
    Request Randomness
    Mine Blocks    6
    ${result} =    Have Node Got Keyword    Transaction successful(fulfill_randomness)    ${NODE_PROCESS_LIST}
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
    ${log_phase_2} =   Get Keyword From Node Log    2    Waiting for Phase 2 to start
    Mine Blocks    9
    ${log_group_available} =       All Nodes Have Keyword    dkg_status transfered from CommitSuccess to WaitForPostProcess    ${NODE_PROCESS_LIST}
    Mine Blocks    20
    Sleep    2s
    Group Node Number Should Be    0    4
    ${group} =   Get Group    0
    ${ckeck_result} =    Check Group Status    ${group}
    
    Mine Blocks    20
    Sleep    2s
    ${node_rewards} =    Get Event    ${NODE_REGISTRY_CONTRACT}    NodeRewarded

    ${result} =    Has Equal Value    ${node_rewards['args']['nodeAddress']}    ${address1}    ${address2}    ${address3}    ${address4}
    Should Be True    ${result}
    Mine Blocks    20
    Sleep    2s
    Deploy User Contract
    Request Randomness
    
    Have Node Got Keyword    send partial signature to committer    ${NODE_PROCESS_LIST}
    Mine Blocks    10
    Sleep    3s
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
    
    ${log_phase_1} =    All Nodes Have Keyword    dkg_status transfered from CommitSuccess to WaitForPostProcess    ${NODE_PROCESS_LIST}
    
    Mine Blocks    20
    Sleep    2s
    ${slash_events} =    Get Events    ${NODE_REGISTRY_CONTRACT}    NodeSlashed
    
    ${result} =    Events Should Contain All Value    ${slash_events}    nodeIdAddress    ${address1}    ${address2}    ${address3}    ${address4}
    Should Be True    ${result}
    ${group} =   Get Group    0
    Should Be Equal As Strings    ${group[7]}    False
    Teardown Scenario Testing Environment

DKG Commit1
    [Documentation]
    ...    1. Given a group of nodes do DKG
    ...    2. When 1, 2 submit their results
    ...    3. 3 quit
    ...    4. Expect 3 be slashed
    Sleep    3s
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s
    ${address1} =    Get Address By Index    1
    ${address2} =    Get Address By Index    2
    ${address3} =    Get Address By Index    3

    ${node3} =    Stake And Run Node    3
    ${log_register} =    Get Keyword From Node Log    3    Transaction successful(node_register)
    Kill Node By Index    3
    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    
    Have Node Got Keyword    Transaction successful(post_process_dkg)    ${NODE_PROCESS_LIST}
    Mine Blocks    20
    Sleep    2s
    ${slash_event} =    Get Event    ${NODE_REGISTRY_CONTRACT}    NodeSlashed
    ${address3} =    To Checksum Address    ${address3}
    Should Be Equal As Strings    ${slash_event['args']['nodeIdAddress']}    ${address3}
    Teardown Scenario Testing Environment

DKG Commit2
    [Documentation]
    ...    1. Given a group of nodes do DKG
    ...    2. When 1 submit their results
    ...    3. 2, 3 quit
    ...    4. Expect 2, 3 be slashed
    Sleep    3s
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s
    ${address1} =    Get Address By Index    1
    ${address2} =    Get Address By Index    2
    ${address3} =    Get Address By Index    3

    ${node3} =    Stake And Run Node    3
    ${log_register} =    Get Keyword From Node Log    3    Transaction successful(node_register)
    Kill Node By Index    3
    
    ${node2} =    Stake And Run Node    2
    ${log_register} =    Get Keyword From Node Log    2    Transaction successful(node_register)
    Kill Node By Index    2

    ${node1} =    Stake And Run Node    1

    Have Node Got Keyword    Transaction successful(post_process_dkg)    ${NODE_PROCESS_LIST}
    Mine Blocks    20
    Sleep    2s
    ${slash_events} =    Get Events    ${NODE_REGISTRY_CONTRACT}    NodeSlashed
    
    ${result} =    Events Should Contain All Value    ${slash_events}    nodeIdAddress    ${address2}    ${address3}
    Teardown Scenario Testing Environment

DKG Commit3
    [Documentation]
    ...    1. Given a group of nodes do DKG
    ...    2. When 1, 2, 3 submit their results
    ...    3. 4 quit
    ...    4. Expect 4 be slashed and gorup 0 has 3 nodes
    Sleep    3s
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s
    ${address1} =    Get Address By Index    1
    ${address2} =    Get Address By Index    2
    ${address3} =    Get Address By Index    3
    ${address4} =    Get Address By Index    4

    ${node3} =    Stake And Run Node    4
    ${log_register} =    Get Keyword From Node Log    4    Transaction successful(node_register)
    Kill Node By Index    4
    
    ${node2} =    Stake And Run Node    3
    ${node2} =    Stake And Run Node    2
    ${node1} =    Stake And Run Node    1

    Have Node Got Keyword    Transaction successful(post_process_dkg)    ${NODE_PROCESS_LIST}
    Mine Blocks    20
    Sleep    2s
    ${slash_events} =    Get Events    ${NODE_REGISTRY_CONTRACT}    NodeSlashed
    
    ${result} =    Events Should Contain All Value    ${slash_events}    nodeIdAddress    ${address4}

    ${group} =   Get Group    0
    Group Node Number Should Be    0    3

    Teardown Scenario Testing Environment

*** Test Cases ***

Run DKG Test cases
    [Tags]    l1
    Repeat Keyword    1    Test Rebalance
    Repeat Keyword    1    DKG Happy Path1
    Repeat Keyword    1    DKG Happy Path2
    Repeat Keyword    1    DKG Sad Path1
    Repeat Keyword    1    DKG Commit1
    Repeat Keyword    1    DKG Commit2
    Repeat Keyword    1    DKG Commit3
