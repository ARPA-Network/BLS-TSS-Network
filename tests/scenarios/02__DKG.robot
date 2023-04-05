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

    ${disqulified_node_staking_after_slash} =    Get Node Staking    ${address4}
    Should Be Equal As Integers    ${disqulified_node_staking_before_slash}    ${disqulified_node_staking_after_slash + 1000}

    Deploy User Contract And Request Randomness
    ${log_received_randomness_task} =    Get Keyword From Log   ${node1}    received new randomness task
    Sleep    5s
    Mine Blocks    6
    Sleep    10s
    Check Randomness
    Kill Node    ${node1}
    Kill Node    ${node2}
    Kill Node    ${node3}
    Kill Node    ${node4}
    Set Global Variable    ${NODE_PROCESS_LIST}    ${EMPTY_LIST}
    Teardown Scenario Testing Environment

DKG Happy Path6
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

    ${node1} =    Add Balance And Run Node    1
    ${node2} =    Add Balance And Run Node    2
    ${node3} =    Add Balance And Run Node    3
    ${node4} =    Add Balance And Run Node    4

    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    8
    ${log_phase_2} =   Get Keyword From Log    ${node2}    Waiting for Phase 2 to start
    Mine Blocks    9
    ${log_group_available} =   Get Keyword From Log    ${node2}    Group index:
    Group Node Number Should Be    0    4
    ${group} =   Get Group    0
    ${ckeck_result} =    Check Group Status    ${group}
    
    Mine Blocks    20
    Sleep    2s
    ${node_rewards_1} =    Get Reward    ${address1}
    ${node_rewards_2} =    Get Reward    ${address2}
    ${node_rewards_3} =    Get Reward    ${address3}
    ${node_rewards_4} =    Get Reward    ${address4}
    ${reward_value} =    Get Value From Env    DKG_POST_PROCESS_REWARD
    ${result} =    Has Equal Value    ${reward_value}    ${node_rewards_1}    ${node_rewards_2}    ${node_rewards_3}    ${node_rewards_4}
    Should Be True    ${result}

    Deploy User Contract And Request Randomness
    ${log_received_randomness_task} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}
    Sleep    2s
    Mine Blocks    6
    Sleep    10s
    Check Randomness
    ${group} =   Get Group    0
    Kill Node    ${node1}
    Kill Node    ${node2}
    Kill Node    ${node3}
    Kill Node    ${node4}
    Teardown Scenario Testing Environment

DKG Sad Path2
    [Documentation]
    ...    1. Given a group of nodes do DKG
    ...    2. When 1, 2, 3, 4 submit their results
    ...    3. 2 nodes submit r, 2 nodes submit r'.
    ...    4. threshold is 3. r and r' submitter are not pass (2<3)
    ...    5. post_process_dkg triggered, punish node 1 2 3 4

    Set Enviorment And Deploy Contract
    Sleep    3s
    ${address1} =    Get Address By Index    1
    ${address2} =    Get Address By Index    2
    ${address3} =    Get Address By Index    3
    ${address4} =    Get Address By Index    4

    ${address4} =    To Checksum Address    ${address4}
    ${disqulified_list} =    Make List    ${address4}
    Set Modified Public Key    ${address3}    ${MODIFIRD_PUB_KEY}
    Set Modified Public Key    ${address4}    ${MODIFIRD_PUB_KEY}

    ${node1} =    Add Balance And Run Node    1
    ${node2} =    Add Balance And Run Node    2
    ${node3} =    Add Balance And Run Node    3
    ${node4} =    Add Balance And Run Node    4

    ${staking1} =    Get Node Staking    ${address1}
    ${staking2} =    Get Node Staking    ${address2}
    ${staking3} =    Get Node Staking    ${address3}
    ${staking4} =    Get Node Staking    ${address4}
    ${group} =   Get Group    0

    ${log_phase_1} =    All Nodes Have Keyword    Waiting for Phase 1 to start    ${NODE_PROCESS_LIST}
    Mine Blocks    8
    ${log_phase_publish} =    All Nodes Have Keyword    Calling contract function publish    ${NODE_PROCESS_LIST}
    Mine Blocks    9
    Sleep    2s
    Mine Blocks    20
    ${log_post_process} =    Have Node Got Keyword    DKGEnded    ${NODE_PROCESS_LIST}
    Sleep    3s

    ${staking1_after_slash} =    Get Node Staking    ${address1}
    ${staking2_after_slash} =    Get Node Staking    ${address2}
    ${staking3_after_slash} =    Get Node Staking    ${address3}
    ${staking4_after_slash} =    Get Node Staking    ${address4}
    ${penalty_amout} =    Get Value From Env    DISQUALIFIED_NODE_PENALTY_AMOUNT
    Should Be Equal As Integers    ${staking1}    ${staking1_after_slash + ${penalty_amout}}
    Should Be Equal As Integers    ${staking2}    ${staking2_after_slash + ${penalty_amout}} 
    Should Be Equal As Integers    ${staking3}    ${staking3_after_slash + ${penalty_amout}}
    Should Be Equal As Integers    ${staking4}    ${staking4_after_slash + ${penalty_amout}}
    
    ${group} =   Get Group    0
    Should Be Equal As Strings    ${group[7]}    False
    Teardown Scenario Testing Environment

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
    Set Global Variable    $NODE_PROCESS_LIST    ${EMPTY_LIST}
    ${node1} =    Add Balance And Run Node    1
    ${node2} =    Add Balance And Run Node    2
    ${node3} =    Add Balance And Run Node    3
    ${node4} =    Add Balance And Run Node    4
    ${node5} =    Add Balance And Run Node    5
    ${commit_result} =    All Nodes Have Keyword    Calling contract function commit_dkg    ${NODE_PROCESS_LIST}

    Sleep    3s
    Group Node Number Should Be    0    5

    ${node6} =    Add Balance And Run Node    6
    ${commit_result} =    All Nodes Have Keyword    Calling contract function commit_dkg    ${NODE_PROCESS_LIST}
    Sleep    5s
    Group Node Number Should Be    0    3
    Group Node Number Should Be    1    3
    Teardown Scenario Testing Environment