*** Settings ***
Documentation       Node Registration Scenarios
Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Variables ***
${userA}    11
${userB}    12
${userC}    13
${nodeA}    1
${nodeB}    2
${nodeC}    3
${Day}      86400
${Delta}    10000000000000000000

*** Keywords ***

Test Staking One Account
    [Documentation]
    ...    1. Given a group is ready for randomeness generation
    ...    2. Stake ARPA for one some accounts
    ...    3. Check the staking
    [Arguments]    ${index}    ${amount}
    
    
    ${address} =    Get Address By Index    ${index}
    ${address} =    To Checksum Address    ${address}
    ${stake_before} =   Get Stake    ${address}

    ${result} =    Add ARPA    ${address}    ${amount}
    ${result} =    Approve ARPA    ${index}    ${amount}
    ${node1} =    Stake ARPA    ${index}    ${amount}
    
    ${stake_after} =   Get Stake    ${address}
    Should Be Equal As Integers    ${stake_after}    ${stake_before + ${amount}}

Add Timestamp Days
    [Arguments]    ${days}
    Set Timestamp Interval    ${Day}
    Mine Blocks    ${days}
    ${block_time} =    Get Timestamp
    Set Timestamp Interval    0

Test Staking
    [Documentation]
    ...    1. Given a group is ready for randomeness generation
    ...    2. Stake ARPA for multiple account
    Set Value To Env    IS_STAKE_USER    false
    Set Enviorment And Deploy Contract
    ${address_A} =    Get Address By Index    ${userA}
    ${address_B} =    Get Address By Index    ${userB}

    ${total_amount_before} =    Get Total Stake Amount
    ${total_amount_before} =    Convert To Number    ${total_amount_before}
    
    
    ${reward_rate} =    Get Reward Rate
    Set Timestamp Interval    0
    Start Stake
    Test Staking One Account    ${userA}    50000000000000000000000
    
    Add Timestamp Days    3

    Test Staking One Account    ${userB}    30000000000000000000000
    
    Add Timestamp Days    3

    Test Staking One Account    ${userA}    50000000000000000000000
    ${reward_rate} =    Get Reward Rate
    ${reward_rate} =    Convert To Number    ${reward_rate}

    Add Timestamp Days    4
   
    ${block_time_cur} =    Get Timestamp
    ${reward_B} =    Get Base Reward    ${address_B}
    ${reward_B} =    Convert To Number    ${reward_B}
    #Assert userB earned reward: rewardRate * (3 days * (30,000/80,000) + 4 days * (30,000/130,000))
    ${clac_reward_B} =    Set Variable    ${reward_rate * (3 * ${Day} * (30000/80000)+ 4 * ${Day} * (30000/130000)) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    ${Delta}
    Should Be True    ${result}

    Unstake ARPA    ${userB}    20000000000000000000000

    Test Staking One Account    ${userB}    30000000000000000000000

    ${locked_amount_A} =    Get Frozen Principal    ${address_B}
    Should Be Equal As Integers    ${locked_amount_A[0][0]}    20000000000000000000000
    ${block_time_cur} =    Convert Timestamp    ${block_time_cur}
    ${due_lock_timestamp} =    Convert To Integer    ${locked_amount_A[1][0]}

    #Assert unlocking amount: userB 20,000 for 14 days
    ${unstake_duration} =    Get Value From Env    UNSTAKE_FREEZING_DURATION
    ${unstake_duration} =    Convert To Integer    ${unstake_duration}
    ${due_lock_timestamp_calc} =    Set Variable    ${block_time_cur + ${unstake_duration}}
    ${result} =    Approximately Equal    ${due_lock_timestamp}    ${due_lock_timestamp_calc}    100
    Should Be True    ${result}

    Add Timestamp Days    3
    
    #Assert userA earned reward:
    # rewardRate * (3 days * (50,000/50,000) + 3 days * (50,000/80,000) + 4 days * (100,000/130,000) + 3 days * (100,000/140,000))
    ${clac_reward_A} =    Set Variable    ${reward_rate * (3 * ${Day} * (50000/50000) + 3 * ${Day} * (50000/80000) + 4 * ${Day} * (100000/130000) + 3 * ${Day} * (100000/140000)) * 0.95}
    ${reward_A} =    Get Base Reward    ${address_A}
    ${reward_A} =    Convert To Number    ${reward_A}
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    ${Delta}
    Should Be True    ${result}

    Unstake ARPA    ${userA}    40000000000000000000000

    #Assert unlocking amount: userB 20,000 for 11 days, userA 40,000 for 14 days
    ${locked_amount_A} =    Get Frozen Principal    ${address_A}
    Should Be Equal As Integers    ${locked_amount_A[0][0]}    40000000000000000000000
    ${due_lock_timestamp} =    Convert To Integer    ${locked_amount_A[1][0]}
    ${block_time_cur} =    Get Timestamp
    ${block_time_cur} =    Convert Timestamp    ${block_time_cur}
    ${due_lock_timestamp_calc} =    Set Variable    ${block_time_cur + ${unstake_duration}}
    ${result} =    Approximately Equal    ${due_lock_timestamp}    ${due_lock_timestamp_calc}    100
    Should Be True    ${result}
    Add Timestamp Days    3

    #Assert userA earned reward: rewardRate * (3 days * (60,000/100,000))
    ${reward_A} =    Get Base Reward    ${address_A}
    ${reward_A} =    Convert To Number    ${reward_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * (3 * ${Day} * (60000/100000)) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    ${Delta}
    Should Be True    ${result}

    #Assert userB earned reward: rewardRate * (3 days * (40,000/140,000) + 3 days * (40,000/100,000))
    ${reward_B} =    Get Base Reward    ${address_B}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * (3 * ${Day} * (40000/140000) + 3 * ${Day} * (40000/100000)) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    ${Delta}
    Should Be True    ${result}

    #Assert unlocking amount: userB 20,000 for 8 days, userA 40,000 for 11 days
    ${locked_amount_A} =    Get Frozen Principal    ${address_A}
    Should Be Equal As Integers    ${locked_amount_A[0][0]}    40000000000000000000000
    ${due_lock_timestamp} =    Convert To Integer    ${locked_amount_A[1][0]}
    ${block_time_cur} =    Get Timestamp
    ${block_time_cur} =    Convert Timestamp    ${block_time_cur}
    ${due_lock_timestamp_calc} =    Set Variable    ${block_time_cur + 11 * ${Day}}
    ${result} =    Approximately Equal    ${due_lock_timestamp}    ${due_lock_timestamp_calc}    100
    Should Be True    ${result}

    ${locked_amount_B} =    Get Frozen Principal    ${address_B}
    Should Be Equal As Integers    ${locked_amount_B[0][0]}    20000000000000000000000
    ${due_lock_timestamp} =    Convert To Integer    ${locked_amount_B[1][0]}
    ${block_time_cur} =    Get Timestamp
    ${block_time_cur} =    Convert Timestamp    ${block_time_cur}
    ${due_lock_timestamp_calc} =    Set Variable    ${block_time_cur + 8 * ${Day}}
    ${result} =    Approximately Equal    ${due_lock_timestamp}    ${due_lock_timestamp_calc}    100
    Should Be True    ${result}

    Unstake ARPA    ${userA}    10000000000000000000000
    
    Add Timestamp Days    4

    #Assert userA earned reward: rewardRate * 4 days * (50,000/90,000)
    ${reward_A} =    Get Base Reward    ${address_A}
    ${reward_A} =    Convert To Number    ${reward_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * 4 * ${Day} * (50000/90000) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    ${Delta}
    Should Be True    ${result}
    #Assert userB earned reward:
    # rewardRate * (3 days * (40,000/140,000) + 3 days * (40,000/100,000) + 4 days * (40,000/90,000))
    ${reward_B} =    Get Base Reward    ${address_B}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * (3 * ${Day} * (40000/140000) + 3 * ${Day} * (40000/100000) + 4 * ${Day} * (40000/90000)) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    ${Delta}
    Should Be True    ${result}

    Unstake ARPA    ${userB}    30000000000000000000000
    Test Staking One Account    ${userA}    50000000000000000000000

    Add Timestamp Days    10
    
    #userA claims claimable:
    # 40,000 + rewardRate * (4 days * (50,000/90,000) + 10 days * (100,000/110,000))
    ${arpa_balance_before_claim_A} =    Get ARPA Balance    ${address_A}
    ${reward_A} =    Get Base Reward    ${address_A}
    ${result} =    Claim Reward    ${userA}
    ${arpa_balance_after_claim_A} =    Get ARPA Balance    ${address_A}
    ${arpa_balance_before_claim_A} =    Convert To Number    ${arpa_balance_before_claim_A}
    ${arpa_balance_after_claim_A} =    Convert To Number    ${arpa_balance_after_claim_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * (4 * ${Day} * (50000/90000) + 10 * ${Day} * (100000/110000)) * 0.95}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_A}    ${arpa_balance_before_claim_A + ${clac_reward_A}}    ${Delta}
    Should Be True    ${result}

    #userB claims claimable:
    # 20,000 + rewardRate * (10 days * (10,000/110,000))
    ${arpa_balance_before_claim_B} =    Get ARPA Balance    ${address_B}
    ${reward_B} =    Get Base Reward    ${address_B}
    ${result} =    Claim Reward    ${userB}
    ${arpa_balance_after_claim_B} =    Get ARPA Balance    ${address_B}
    ${arpa_balance_before_claim_B} =    Convert To Number    ${arpa_balance_before_claim_B}
    ${arpa_balance_after_claim_B} =    Convert To Number    ${arpa_balance_after_claim_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * (10 * ${Day} * (10000/110000)) * 0.95}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_B}    ${arpa_balance_before_claim_B + ${clac_reward_B}}    ${Delta}
    Should Be True    ${result}

    Teardown Scenario Testing Environment

Test Staking With Node
    Set Value To Env    IS_STAKE_USER    false
    Set Enviorment And Deploy Contract
    ${address_userA} =    Get Address By Index    ${userA}
    ${address_userB} =    Get Address By Index    ${userB}
    ${address_userC} =    Get Address By Index    ${userC}
    ${address_nodeA} =    Get Address By Index    ${nodeA}
    ${address_nodeB} =    Get Address By Index    ${nodeB}
    ${address_nodeC} =    Get Address By Index    ${nodeC}
    
    #T0
    Set Timestamp Interval    0
    Start Stake
    ${reward_rate} =    Get Reward Rate
    ${reward_rate} =    Convert To Number    ${reward_rate}

    Test Staking One Account    ${userA}    3000000000000000000000
    ${result} =    Stake And Run Node    ${nodeA}
    ${result} =        Get Keyword From Log    ${nodeA}    Transaction successful(node_register)

    #T5
    Add Timestamp Days    5

    Test Staking One Account    ${userB}    10000000000000000000000
    ${result} =    Stake And Run Node    ${nodeB}
    ${result} =        Get Keyword From Log    ${nodeB}    Transaction successful(node_register)                        

    ${total_community_stake} =    Get Community Stake Count
    Should Be Equal As Integers    ${total_community_stake}    13000000000000000000000
    #T12
    Add Timestamp Days    7
    Test Staking One Account    ${userC}    284000000000000000000000
    ${total_community_stake} =    Get Community Stake Count
    Should Be Equal As Integers    ${total_community_stake}    297000000000000000000000
    #T15
    Add Timestamp Days    3
    #Assert userA earned reward:
    # rewardRate * ((3,000/3,000) * 5 days + (3,000/13,000) * 7 days + (3,000/297,000) * 3 days )
    ${reward_A} =    Get Base Reward    ${address_userA}
    ${reward_A} =    Convert To Number    ${reward_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * ((3000/3000) * 5 * ${Day} + (3000/13000) * 7 * ${Day} + (3000/297000) * 3 * ${Day}) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    ${Delta}
    Should Be True    ${result}
    
    Unstake ARPA    ${userA}    3000000000000000000000
    Test Staking One Account    ${userB}    5000000000000000000000
    ${total_community_stake} =    Get Community Stake Count
    Should Be Equal As Integers    ${total_community_stake}    299000000000000000000000
    #T17
    Add Timestamp Days    2
    #Assert nodeA earned reward: rewardRate * 5% * (5 days + 12 days/2)
    ${reward_A} =    Get Delegation Reward    ${address_nodeA}
    ${reward_A} =    Convert To Number    ${reward_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * 0.05 * (5 * ${Day} + 12 * ${Day}/2)}
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    ${Delta}
    Should Be True    ${result}

    Test Staking One Account    ${userA}    20000000000000000000000
    ${quit_result} =    Node Quit    ${nodeA}
    ${unsatke_result} =    Unstake ARPA    ${nodeA}    50000000000000000000000
    ${stake_after} =   Get Stake     ${address_nodeA}
    Should Be Equal As Integers    ${stake_after}    0
    ${total_community_stake} =    Get Community Stake Count
    Should Be Equal As Integers    ${total_community_stake}    319000000000000000000000

    #Assert unlocking amount: userA 3,000 for 12 days
    ${locked_amount_A} =    Get Frozen Principal    ${address_userA}
    Should Be Equal As Integers    ${locked_amount_A[0][0]}    3000000000000000000000
    ${due_lock_timestamp} =    Convert To Integer    ${locked_amount_A[1][0]}
    ${block_time_cur} =    Get Timestamp
    ${block_time_cur} =    Convert Timestamp    ${block_time_cur}
    ${due_lock_timestamp_calc} =    Set Variable    ${block_time_cur + 12 * ${Day}}
    ${result} =    Approximately Equal    ${due_lock_timestamp}    ${due_lock_timestamp_calc}    100
    Should Be True    ${result}
    #T20
    Add Timestamp Days    3

    #Assert userC earned reward:
    # rewardRate * ((284,000/297,000) * 3 days + (284,000/299,000) * 2 days + (284,000/319000) * 3 days)
    ${reward_C} =    Get Base Reward    ${address_userC}
    ${reward_C} =    Convert To Number    ${reward_C}
    ${clac_reward_C} =    Set Variable    ${reward_rate * ((284000/297000) * 3 * ${Day} + (284000/299000) * 2 * ${Day} + (284000/319000) * 3 * ${Day}) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_C}    ${reward_C}    ${Delta}
    Should Be True    ${result}

    ${result} =    Unstake ARPA    ${userC}    50000000000000000000000
    ${result} =    Stake And Run Node    ${nodeC}
    ${result} =        Get Keyword From Log    ${nodeC}    Transaction successful(node_register)
    ${total_community_stake} =    Get Community Stake Count
    Should Be Equal As Integers    ${total_community_stake}    269000000000000000000000
    #T26
    Add Timestamp Days    6

    #userA: rewardRate * ((20,000/319000) * 3 days + (20,000/269000) * 6 days)
    ${reward_A} =    Get Base Reward    ${address_userA}
    ${reward_A} =    Convert To Number    ${reward_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * ((20000/319000) * 3 * ${Day} + (20000/269000) * 6 * ${Day}) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    ${Delta}
    Should Be True    ${result}

    #userB: rewardRate * ((10,000/13,000) * 7 days + (10,000/297,000) * 3 days +
    # (15,000/299,000) * 2 days + (15,000/319000) * 3 days + (15,000/269000) * 6 days)
    ${reward_B} =    Get Base Reward    ${address_userB}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * ((10000/13000) * 7 * ${Day} + (10000/297000) * 3 * ${Day} + (15000/299000) * 2 * ${Day} + (15000/319000) * 3 * ${Day} + (15000/269000) * 6 * ${Day}) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    ${Delta}
    Should Be True    ${result}

    #userC: rewardRate * (234,000/269000) * 6 days
    ${reward_C} =    Get Base Reward    ${address_userC}
    ${reward_C} =    Convert To Number    ${reward_C}
    ${clac_reward_C} =    Set Variable    ${reward_rate * (234000/269000) * 6 * ${Day} * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_C}    ${reward_C}    ${Delta}
    Should Be True    ${result}

    #nodeB: rewardRate * 5% * (12 days/2 + 3 days + 6 days/2)
    ${reward_B} =    Get Delegation Reward    ${address_nodeB}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * 0.05 * (12 * ${Day}/2 + 3 * ${Day} + 6 * ${Day}/2)}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    ${Delta}
    Should Be True    ${result}
    
    #T29
    Add Timestamp Days    3
    #userA claim claimable (unlocked staking amount + earned reward ):
    # 3,000 + rewardRate * ((20,000/319000) * 3 days + (20,000/269000) * 9 days)
    ${arpa_balance_before_claim_A} =    Get ARPA Balance    ${address_userA}
    ${reward_A} =    Get Base Reward    ${address_userA}
    ${result} =    Claim    ${userA}
    ${arpa_balance_after_claim_A} =    Get ARPA Balance    ${address_userA}
    ${arpa_balance_before_claim_A} =    Convert To Number    ${arpa_balance_before_claim_A}
    ${arpa_balance_after_claim_A} =    Convert To Number    ${arpa_balance_after_claim_A}
    ${clac_reward_A} =    Set Variable    ${3000000000000000000000 + ${reward_rate} * ((20000/319000) * 3 * ${Day} + (20000/269000) * 9 * ${Day}) * 0.95}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_A}    ${arpa_balance_before_claim_A + ${clac_reward_A} + 3000}    ${Delta}

    #T30
    Add Timestamp Days    1
    #userA: rewardRate * ((20,000/269000) * 1 day)
    ${reward_A} =    Get Base Reward    ${address_userA}
    ${reward_A} =    Convert To Number    ${reward_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * ((20000/269000) * 1 * ${Day}) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    ${Delta}
    Should Be True    ${result}

    #userB: rewardRate * ((10,000/13,000) * 7 days + (10,000/297,000) * 3 days +
    # (15,000/299,000) * 2 days + (15,000/319000) * 3 days + (15,000/269000) * 10 days)
    ${reward_B} =    Get Base Reward    ${address_userB}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * ((10000/13000) * 7 * ${Day} + (10000/297000) * 3 * ${Day} + (15000/299000) * 2 * ${Day} + (15000/319000) * 3 * ${Day} + (15000/269000) * 10 * ${Day}) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    ${Delta}
    Should Be True    ${result}

    #userC: rewardRate * (234,000/269000) * 10 days
    ${reward_C} =    Get Base Reward    ${address_userC}
    ${reward_C} =    Convert To Number    ${reward_C}
    ${clac_reward_C} =    Set Variable    ${reward_rate * (234000/269000) * 10 * ${Day} * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_C}    ${reward_C}    ${Delta}
    Should Be True    ${result}

    #nodeB: rewardRate * 5% * (12 days/2 + 3 days + 10 days/2)
    ${reward_B} =    Get Delegation Reward    ${address_nodeB}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * 0.05 * (12 * ${Day}/2 + 3 * ${Day} + 10 * ${Day}/2)}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    ${Delta}
    Should Be True    ${result}

    #T36
    Add Timestamp Days    6
    #userA: rewardRate * ((20,000/269000) * 1 day)
    ${reward_A} =    Get Base Reward    ${address_userA}
    ${reward_A} =    Convert To Number    ${reward_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * ((20000/269000) * 1 * ${Day}) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    ${Delta}
    Should Be True    ${result}

    #userB: rewardRate * ((10,000/13,000) * 7 days + (10,000/297,000) * 3 days +
    # (15,000/299,000) * 2 days + (15,000/319000) * 3 days + (15,000/269000) * 10 days)
    ${reward_B} =    Get Base Reward    ${address_userB}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * ((10000/13000) * 7 * ${Day} + (10000/297000) * 3 * ${Day} + (15000/299000) * 2 * ${Day} + (15000/319000) * 3 * ${Day} + (15000/269000) * 10 * ${Day}) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    ${Delta}
    Should Be True    ${result}

    #userC: rewardRate * (234,000/269000) * 10 days
    ${reward_C} =    Get Base Reward    ${address_userC}
    ${reward_C} =    Convert To Number    ${reward_C}
    ${clac_reward_C} =    Set Variable    ${reward_rate * (234000/269000) * 10 * ${Day} * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_C}    ${reward_C}    ${Delta}
    Should Be True    ${result}

    #nodeB: rewardRate * 5% * (12 days/2 + 3 days + 10 days/2)
    ${reward_B} =    Get Delegation Reward    ${address_nodeB}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * 0.05 * (12 * ${Day}/2 + 3 * ${Day} + 10 * ${Day}/2)}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    1000000000000000000
    Should Be True    ${result}

    #userC claim claimable (unlocked staking amount + earned reward ): 50,000 + T30 userC earned reward
    ${arpa_balance_before_claim_C} =    Get ARPA Balance    ${address_userC}
    ${result} =    Claim    ${userC}
    ${arpa_balance_after_claim_C} =    Get ARPA Balance    ${address_userC}
    ${arpa_balance_before_claim_C} =    Convert To Number    ${arpa_balance_before_claim_C}
    ${arpa_balance_after_claim_C} =    Convert To Number    ${arpa_balance_after_claim_C}
    ${clac_reward_C} =    Set Variable    ${50000000000000000000000 + ${reward_rate} * (234000/269000) * 10 * ${Day} * 0.95}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_C}    ${arpa_balance_before_claim_C + ${clac_reward_C} + 50000}    ${Delta}
    Should Be True    ${result}

    #T40
    Add Timestamp Days    4
    Unstake ARPA    ${userA}    20000000000000000000000
    Unstake ARPA    ${userB}    15000000000000000000000
    Unstake ARPA    ${userC}    23400000000000000000000
    ${quit_result} =    Node Quit    ${nodeB}
    Unstake ARPA    ${nodeB}    50000000000000000000000
    ${quit_result} =    Node Quit    ${nodeC}
    Unstake ARPA    ${nodeC}    50000000000000000000000

    #T54
    Add Timestamp Days    14
    #userA claim 20,000
    ${arpa_balance_before_claim_A} =    Get ARPA Balance    ${address_userA}
     ${result} =    Claim    ${userA}
    ${arpa_balance_after_claim_A} =    Get ARPA Balance    ${address_userA}
    ${arpa_balance_before_claim_A} =    Convert To Number    ${arpa_balance_before_claim_A}
    ${arpa_balance_after_claim_A} =    Convert To Number    ${arpa_balance_after_claim_A}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_A}    ${arpa_balance_before_claim_A + 20000000000000000000000}    ${Delta}
    Should Be True    ${result}

    #userB claim 15,000
    ${arpa_balance_before_claim_B} =    Get ARPA Balance    ${address_userB}
    ${result} =    Claim    ${userB}
    ${arpa_balance_after_claim_B} =    Get ARPA Balance    ${address_userB}
    ${arpa_balance_before_claim_B} =    Convert To Number    ${arpa_balance_before_claim_B}
    ${arpa_balance_after_claim_B} =    Convert To Number    ${arpa_balance_after_claim_B}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_B}    ${arpa_balance_before_claim_B + 15000000000000000000000}    ${Delta}
    Should Be True    ${result}

    #userC claim 23,4000
    ${arpa_balance_before_claim_C} =    Get ARPA Balance    ${address_userC}
    ${result} =    Claim    ${userC}
    ${arpa_balance_after_claim_C} =    Get ARPA Balance    ${address_userC}
    ${arpa_balance_before_claim_C} =    Convert To Number    ${arpa_balance_before_claim_C}
    ${arpa_balance_after_claim_C} =    Convert To Number    ${arpa_balance_after_claim_C}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_C}    ${arpa_balance_before_claim_C + 23400000000000000000000}    ${Delta}
    Should Be True    ${result}

    #nodeB claim 50,000
    ${arpa_balance_before_claim_B} =    Get ARPA Balance    ${address_nodeB}
    ${result} =    Claim Frozen Principal    ${nodeB}
    ${arpa_balance_after_claim_B} =    Get ARPA Balance    ${address_nodeB}
    ${arpa_balance_before_claim_B} =    Convert To Number    ${arpa_balance_before_claim_B}
    ${arpa_balance_after_claim_B} =    Convert To Number    ${arpa_balance_after_claim_B}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_B}    ${arpa_balance_before_claim_B + 50000000000000000000000}    ${Delta}
    Should Be True    ${result}

    #nodeC claim 50,000
    ${arpa_balance_before_claim_C} =    Get ARPA Balance    ${address_nodeC}
    ${result} =    Claim Frozen Principal    ${nodeC}
    ${arpa_balance_after_claim_C} =    Get ARPA Balance    ${address_nodeC}
    ${arpa_balance_before_claim_C} =    Convert To Number    ${arpa_balance_before_claim_C}
    ${arpa_balance_after_claim_C} =    Convert To Number    ${arpa_balance_after_claim_C}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_C}    ${arpa_balance_before_claim_C + 50000000000000000000000}    ${Delta}
    Should Be True    ${result}

*** Test Cases ***

Run Test Cases
    Repeat Keyword    1    Test Staking
    Repeat Keyword    1    Test Staking With Node