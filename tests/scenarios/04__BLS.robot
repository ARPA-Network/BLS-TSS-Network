*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Keywords ***

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


    ${log_group_available} =       All Nodes Have Keyword    Group index:0 epoch:3 is available    ${NODE_PROCESS_LIST}

    ${node6} =    Stake And Run Node    6
    ${log_group_0} =    Have Node Got Keyword    Group index:0 epoch:4 is available    ${NODE_PROCESS_LIST}
    ${log_grouo_1} =    Have Node Got Keyword    Group index:1 epoch:1 is available    ${NODE_PROCESS_LIST}
    Mine Blocks    20
    Sleep    3s
    Deploy User Contract
    ${current_randomness} =    Set Variable    1
    ${cur_block} =    Convert To Integer    0
    ${last_group} =    Convert To Integer    -1
    ${cur_group} =    Convert To Integer    -2
    WHILE    ${cur_block < 400}
        ${cur_block} =    Get Latest Block Number
        Mine Blocks    10
        Request Randomness
        Mine Blocks    10
        Sleep    5s
        ${last_group} =    Set Variable    ${cur_group}
        ${current_randomness} =    Check Randomness
        ${event} =    Get Latest Event    ${ADAPTER_CONTRACT}    RandomnessRequestResult        ${cur_block}
        ${cur_group} =    Set Variable    ${event['args']['groupIndex']}
        Should Not Be Equal As Strings    ${cur_group}    ${last_group}
        Mine Blocks    10
        Sleep   3s
        ${cur_block} =    Convert To Integer    ${cur_block}
    END

    Set Global Variable    ${NODE_PROCESS_LIST}    ${EMPTY_LIST}
    Teardown Scenario Testing Environment

BLS Happy Path2
    [Documentation]
    ...    1. Given there is a BLS task pending
    ...    2. When committer 1/2/3 fulfill randomness
    ...    3. Then check the caller is committer
    ...    4. Then reward the participants in the group who execute the task,
    ...    and give reward to the committer
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s

    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${node4} =    Stake And Run Node    4

    ${log_group_available} =       All Nodes Have Keyword    Group index:0 epoch:2 is available    ${NODE_PROCESS_LIST}
    
    Sleep    3s
    ${request_id} =    Deploy User Contract
    Clear Log
    ${request_id} =    Request Randomness
    ${start_block} =    Get Latest Block Number
    mine_blocks    10
    Sleep    3s
    ${index} =    Convert To Integer    1
    ${count} =    Convert To Integer    0
    ${final_committer} =    Set Variable    0
    WHILE    ${index} < 5
        ${log_calling_commit} =    Get Keyword From Log    ${index}    Calling contract transaction fulfill_randomnes
        IF  ${log_calling_commit != None}
            ${count} =    Set Variable    ${count + 1}
            ${node_address} =    Get Address By Index    ${index}
            ${is_committer} =    Is Committer    0    ${node_address}
            Should Be True    ${is_committer}
            ${commit_success} =    Get Keyword From Log    ${index}    Transaction successful(fulfill_randomness)
            IF  ${commit_success != None}
                ${final_committer} =    Set Variable    ${node_address}
            END 
        END
        
        ${index} =    Set Variable    ${index + 1}
    END
    Should Be True    ${count >= 1} 
    Should Be True    ${final_committer != 0}
    
    ${node_rewards} =    get_events    ${CONTROLLER_CONTRACT}    NodeRewarded    ${start_block}
    Log    ${node_rewards}
    ${final_committer_reward} =    Get Amount Count From Reward Events    ${node_rewards}    ${final_committer}

    Set Global Variable    ${NODE_PROCESS_LIST}    ${EMPTY_LIST}
    Teardown Scenario Testing Environment

Corner Case1
    [Documentation]
    ...    1. Given Adapter think group0 group1 work well
    ...    2. Given group0 size = 3, threshold = 3, but only 2 nodes can work
    ...    3. Given group1 size = 3, threshold = 3, and 3 nodes can work
    ...    4. Given Adapter assign BLS task to group0, group0 can not finish in time
    ...    5. When group1 finish, group1 committer call fulfill randomness
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s

    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${node4} =    Stake And Run Node    4
    ${node5} =    Stake And Run Node    5

    ${log_group_available} =       All Nodes Have Keyword    Group index:0 epoch:3 is available    ${NODE_PROCESS_LIST}

    ${node6} =    Stake And Run Node    6
    ${log_group_0} =    Have Node Got Keyword    Group index:0 epoch:4 is available    ${NODE_PROCESS_LIST}
    ${log_grouo_1} =    Have Node Got Keyword    Group index:1 epoch:1 is available    ${NODE_PROCESS_LIST}
    
    Sleep    10s
    ${group0} =    Get Group    0
    ${nodes} =    Set Variable    ${group0[5]}
    ${node_addrss} =    Set Variable    ${nodes[0]}
    ${node_index} =    Get Index By Address    ${node_addrss}
    Kill Node By Index    ${node_index}

    Deploy User Contract
    Request Randomness
    Mine Blocks    10
    Sleep    10s
    ${events} =    Get Events    ${ADAPTER_CONTRACT}    RandomnessRequestResult
    ${result} =    Events Values Should Be    ${events}    groupIndex    1
    Teardown Scenario Testing Environment

Corner Case2
    [Documentation]
    ...    1. Given Adapter think group0 group1 work well
    Compile Proto
    Set Global Variable    $BLOCK_TIME    1
    Set Enviorment And Deploy Contract
    Sleep    3s

    ${node1} =    Stake And Run Node    1
    ${node2} =    Stake And Run Node    2
    ${node3} =    Stake And Run Node    3
    ${node4} =    Stake And Run Node    4
    ${node5} =    Stake And Run Node    5
    ${log_group_available} =       All Nodes Have Keyword    Group index:0 epoch:3 is available    ${NODE_PROCESS_LIST}
    ${node6} =    Stake And Run Node    6
    ${log_group_0} =    Have Node Got Keyword    Group index:0 epoch:4 is available    ${NODE_PROCESS_LIST}
    ${log_grouo_1} =    Have Node Got Keyword    Group index:1 epoch:1 is available    ${NODE_PROCESS_LIST}

    Mine Blocks    20
    Sleep    3s
    Deploy User Contract
    Request Randomness
    Mine Blocks    10
    Sleep    3s
    Clear Log
    ${task_type} =    Convert To Integer    6
    ${group1} =    Get Group    1
    ${group1_nodes} =    Set Variable    ${group1[5]}
    ${group1_index_0} =    Get Index By Address    ${group1_nodes[0]}
    Shutdown Listener    ${group1_index_0}    ${task_type}
    ${group1_index_1} =    Get Index By Address    ${group1_nodes[1]}
    Shutdown Listener    ${group1_index_1}    ${task_type}
    ${group1_index_2} =    Get Index By Address    ${group1_nodes[2]}
    Shutdown Listener    ${group1_index_2}    ${task_type}

    ${group0} =    Get Group    0
    ${one_node_in_group0} =    Set Variable    ${group0[5][0]}
    ${node_index} =    Get Index By Address    ${one_node_in_group0}

    ${cur_block} =    Get Latest Block Number
    Deploy User Contract
    ${request_id} =    Request Randomness
    Get Keyword From Log    ${node_index}    send partial signature to committer

    Start Listener    ${group1_index_0}    ${task_type}
    Start Listener    ${group1_index_1}    ${task_type}
    Start Listener    ${group1_index_2}    ${task_type}
    
    Sleep    10s
    ${events} =    Get Events    ${ADAPTER_CONTRACT}    RandomnessRequestResult    ${cur_block}
    ${result} =    Events Values Should Be    ${events}    groupIndex    1
    Teardown Scenario Testing Environment

*** Test Cases ***

Run BLS Test Cases
    Repeat Keyword    1    BLS Happy Path1
    Repeat Keyword    1    BLS Happy Path2
    Repeat Keyword    1    Corner Case1
    Repeat Keyword    1    Corner Case2