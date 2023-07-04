*** Settings ***
Documentation       Node Registration Scenarios

Library             src/environment/contract.py
Library             src/environment/log.py
Resource            src/common.resource
Resource            src/contract.resource
Resource            src/node.resource

*** Variables ***
${A}    11
${B}    12
${Day}    86400

*** Keywords ***

Test Staking One Node
    [Documentation]
    ...    1. Given a group is ready for randomeness generation
    ...    2. Stake ARPA for one some accounts
    ...    3. Check the staking
    [Arguments]    ${index}    ${amount}
    
    ${address} =    Get Address By Index    ${index}
    ${address} =    To Checksum Address    ${address}

    ${result} =    Add ARPA    ${address}    ${amount}
    ${result} =    Approve ARPA    ${index}    ${amount}
    ${node1} =    Stake ARPA    ${index}    ${amount}
    
    ${stake} =   Get Stake    ${address}

Add Timestamp Days
    [Arguments]    ${days}
    Set Timestamp Interval    ${Day}
    Mine Blocks    ${days}
    ${block_time} =    Get Timestamp
    Set Timestamp Interval    0

Test Staking
    [Documentation]
    ...    1. Given a group is ready for randomeness generation
    ...    2. Stake ARPA for multiple nodes
    Set Value To Env    IS_STAKE_USER    false
    Set Enviorment And Deploy Contract
    ${address_A} =    Get Address By Index    11
    ${address_B} =    Get Address By Index    12

    ${total_amount_before} =    Get Total Stake Amount
    ${total_amount_before} =    Convert To Number    ${total_amount_before}
    
    
    ${reward_rate} =    Get Reward Rate
    Set Timestamp Interval    0
    Start Stake
    Test Staking One Node    ${A}    50000000000000000000000
    
    Add Timestamp Days    3

    Test Staking One Node    ${B}    30000000000000000000000
    
    Add Timestamp Days    3

    Test Staking One Node    ${A}    50000000000000000000000
    ${reward_rate} =    Get Reward Rate
    ${reward_rate} =    Convert To Number    ${reward_rate}

    Add Timestamp Days    4
   
    ${block_time_cur} =    Get Timestamp
    ${reward_B} =    Get Base Reward    ${address_B}
    ${reward_B} =    Convert To Number    ${reward_B}
    #Assert userB earned reward: rewardRate * (3 days * (30,000/80,000) + 4 days * (30,000/130,000))
    ${clac_reward_B} =    Set Variable    ${reward_rate * (3 * ${Day} * (30000/80000)+ 4 * ${Day} * (30000/130000)) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    1000000000000000000
    Should Be True    ${result}

    Unstake ARPA    ${B}    20000000000000000000000

    Test Staking One Node    ${B}    30000000000000000000000

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
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    1000000000000000000
    Should Be True    ${result}

    Unstake ARPA    ${A}    40000000000000000000000

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
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    1000000000000000000
    Should Be True    ${result}

    #Assert userB earned reward: rewardRate * (3 days * (40,000/140,000) + 3 days * (40,000/100,000))
    ${reward_B} =    Get Base Reward    ${address_B}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * (3 * ${Day} * (40000/140000) + 3 * ${Day} * (40000/100000)) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    1000000000000000000
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

    Unstake ARPA    ${A}    10000000000000000000000
    
    Add Timestamp Days    4

    #Assert userA earned reward: rewardRate * 4 days * (50,000/90,000)
    ${reward_A} =    Get Base Reward    ${address_A}
    ${reward_A} =    Convert To Number    ${reward_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * 4 * ${Day} * (50000/90000) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_A}    ${reward_A}    1000000000000000000
    Should Be True    ${result}
    #Assert userB earned reward:
    # rewardRate * (3 days * (40,000/140,000) + 3 days * (40,000/100,000) + 4 days * (40,000/90,000))
    ${reward_B} =    Get Base Reward    ${address_B}
    ${reward_B} =    Convert To Number    ${reward_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * (3 * ${Day} * (40000/140000) + 3 * ${Day} * (40000/100000) + 4 * ${Day} * (40000/90000)) * 0.95}
    ${result} =    Approximately Equal    ${clac_reward_B}    ${reward_B}    1000000000000000000
    Should Be True    ${result}

    Unstake ARPA    ${B}    30000000000000000000000
    Test Staking One Node    ${A}    50000000000000000000000

    Add Timestamp Days    10
    
    #userA claims claimable:
    # 40,000 + rewardRate * (4 days * (50,000/90,000) + 10 days * (100,000/110,000))
    ${arpa_balance_before_claim_A} =    Get ARPA Balance    ${address_A}
    ${reward_A} =    Get Base Reward    ${address_A}
    ${result} =    Claim Reward    ${A}
    ${arpa_balance_after_claim_A} =    Get ARPA Balance    ${address_A}
    ${arpa_balance_before_claim_A} =    Convert To Number    ${arpa_balance_before_claim_A}
    ${arpa_balance_after_claim_A} =    Convert To Number    ${arpa_balance_after_claim_A}
    ${clac_reward_A} =    Set Variable    ${reward_rate * (4 * ${Day} * (50000/90000) + 10 * ${Day} * (100000/110000)) * 0.95}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_A}    ${arpa_balance_before_claim_A + ${clac_reward_A}}    1000000000000000000
    Should Be True    ${result}

    #userB claims claimable:
    # 20,000 + rewardRate * (10 days * (10,000/110,000))
    ${arpa_balance_before_claim_B} =    Get ARPA Balance    ${address_B}
    ${reward_B} =    Get Base Reward    ${address_B}
    ${result} =    Claim Reward    ${B}
    ${arpa_balance_after_claim_B} =    Get ARPA Balance    ${address_B}
    ${arpa_balance_before_claim_B} =    Convert To Number    ${arpa_balance_before_claim_B}
    ${arpa_balance_after_claim_B} =    Convert To Number    ${arpa_balance_after_claim_B}
    ${clac_reward_B} =    Set Variable    ${reward_rate * (10 * ${Day} * (10000/110000)) * 0.95}
    ${result} =    Approximately Equal    ${arpa_balance_after_claim_B}    ${arpa_balance_before_claim_B + ${clac_reward_B}}    1000000000000000000
    Should Be True    ${result}

    Teardown Scenario Testing Environment

*** Test Cases ***

Run Test Cases
    Repeat Keyword    1    Test Staking