*** Settings ***
Documentation       This resource file sets the constants and variables
...                 that applies to all test suites and test cases

Library             Process
Library             String
Library             OperatingSystem
Library             environment/contract.py
Library             environment/node.py
Library             environment/log.py
Library             environment/util.py

Resource            common.resource

*** Variables ***
# Proxy contract:
${PROXY_CONTRACT_DEPLOYER_KEY}           0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
${PROXY_CONTRACT_ADDRESS}
${PROXY_OUTPUT}                          contracts/out/
${PROXY_CONTRACT}
${CONTROLLER_CONTRACT}
${STAKE_CONTRACT}
${ADAPTER_CONTRACT}
${USER_CONTRACT_ADDRESS}                 NULL
${USER_CONTRACT}
# Test value:
${MODIFIRD_PUB_KEY}                      0x137bde2a3eca9e26d5023c8a31c7db75db47b4d1776efc144bc9cfa36403125510292172c806e0d9dd29958c8b359ea9c693179c505558cc95ca8ce6a690eb800652ce1fadb1895c06e5f28e871d8e3797f749941108195d2106a782464a09ed23ece01e5c6512317cd413fbecc36032ab7ba45f62704e9808ec2b6a2dd03d8c
${MODIFIRD_PARTIAL_PUB_KEY}              0x02
@{MODIFIRD_DISQUALIFIED_NODES}           0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC

*** Keywords ***

Set Enviorment And Deploy Contract
    [Documentation]    Set enviorment and deploy proxy contract
    Clear Log
    Setup Chain Environment
    ${result} =    Exec Script    ControllerScenarioTest.s.sol:ControllerScenarioTest
    ${result} =    Exec Script    StakeOperatorScenarioTest.s.sol:StakeOperatorScenarioTestScript
    Bound Contract ABI
    Set ARPA Address
    ${adapter_address} =    Get Value From Env    ADAPTER_ADDRESS
    ${result} =    Create Node Config    ${PROXY_CONTRACT_ADDRESS}    ${adapter_address}
    Kill Previous Node    20
    Clear Database
    Set Global Variable    $NODE_PROCESS_LIST    ${EMPTY_LIST}

Init Envioirmnet
    [Documentation]    Init enviorment
    Build Contract

Build Contract
    [Documentation]    Build contract
    ${result} =    Run    cd contracts; forge build -o out; cd ..
    RETURN    ${result}

Bound Contract ABI
    [Documentation]    Bound contract ABI to handle contract function call
    ${proxy_address} =    Get Value From Env    PROXY_ADDRESS
    ${proxy_address} =    To Checksum Address    ${proxy_address}
    Set Global Variable    $PROXY_CONTRACT_ADDRESS    ${proxy_address}
    ${proxy_contract} =    Get Contract    ${PROXY_OUTPUT}ControllerProxy.sol/ControllerProxy.json    ${PROXY_CONTRACT_ADDRESS}
    Set Global Variable    $PROXY_CONTRACT   ${proxy_contract}
    ${controller_contract} =    Get Contract    ${PROXY_OUTPUT}Controller.sol/Controller.json    ${PROXY_CONTRACT_ADDRESS}
    Set Global Variable    $CONTROLLER_CONTRACT   ${controller_contract}
    ${stake_address} =    Get Value From Env    STAKING_ADDRESS
    ${stake_address} =    To Checksum Address    ${stake_address}
    ${stake_contract} =    Get Contract    ${PROXY_OUTPUT}Staking.sol/Staking.json    ${stake_address}
    Set Global Variable    $STAKE_CONTRACT   ${stake_contract}
    ${adapter_address} =    Get Value From Env    ADAPTER_ADDRESS
    ${adapter_address} =    To Checksum Address    ${adapter_address}
    ${adapter_contract} =    Get Contract    ${PROXY_OUTPUT}Adapter.sol/Adapter.json    ${adapter_address}
    Set Global Variable    $ADAPTER_CONTRACT   ${adapter_contract}

Deploy User Contract
    [Documentation]    Deploy user request contract
    ${contract_output} =    Exec Script    GetRandomNumberScenarioTest.s.sol:GetRandomNumberScenarioTestScript
    ${contract_address} =    Get Contract Address From File    contracts/broadcast/GetRandomNumberScenarioTest.s.sol/31337/run-latest.json
    Set Global Variable    $USER_CONTRACT_ADDRESS    ${contract_address[0]}  
    ${user_contract} =    Get Contract    ${PROXY_OUTPUT}GetRandomNumberExample.sol/GetRandomNumberExample.json    ${user_contract_address}
    Set Global Variable    $USER_CONTRACT   ${user_contract}

Request Randomness
    [Documentation]    Request randomness
    ${request_id} =    Contract Function Transact    ${USER_CONTRACT}    getRandomNumber
    RETURN    ${request_id}

Check Randomness
    [Documentation]    Check the randomness
    ${last_output} =    Run    cast call ${USER_CONTRACT_ADDRESS} "lastRandomnessResult()(uint256)"
    Log    ${last_output}
    RETURN    ${last_output}

Set ARPA Address
    ${address} =    Get Value From Env    ARPA_ADDRESS
    Set Global Variable    $ARPA_CONTRACT_ADDRESS    ${address}

Get Modified CommitdDkg
    [Documentation]    Get all modifed value of commitDkg
    [Arguments]    ${node}
    ${node} =    To Checksum Address    ${node}
    ${result} =    Contract Function Call    ${PROXY_CONTRACT}    getModifiedDkgData    ${node}
    Log    Modified dkg data: ${result}
    RETURN    ${result}

Get Test Node Address
    [Documentation]    Get test node address
    ${result} =    Contract Function Call    ${PROXY_CONTRACT}    getTestAddress
    RETURN    ${result}

Set Modified CommitdDkg
    [Documentation]    Set all modifed value of commitDkg
    [Arguments]    ${node}
    ${node} =    To Checksum Address    ${node}
    ${result} =    Set Modified Public Key    ${node}    ${MODIFIRD_PUB_KEY}
    ${result} =    Set Modified Partial Public Key    ${node}    ${MODIFIRD_PARTIAL_PUB_KEY}
    ${result} =    Set Modified Disqualified Nodes    ${node}    ${MODIFIRD_DISQUALIFIED_NODES}
    Sleep    1s
    ${result} =    Get Modified CommitdDkg    ${node}
    ${result_list} =    Convert To String    ${result[3]}
    Should Be Equal    ${result_list}    [True, True, True]

Set Modified Public Key
    [Documentation]    Set modifed public key of commitDkg
    [Arguments]    ${node}    ${pub_key}
    ${node} =    To Checksum Address    ${node}
    ${result} =    Contract Function Transact    ${PROXY_CONTRACT}    setModifiedPublicKey
    ...    ${node}    ${pub_key}
    Sleep    1s
    ${result} =    Contract Function Call    ${PROXY_CONTRACT}    getModifiedDkgData    ${node}
    Log    Set node ${node} public key: ${pub_key}

Set Modified Partial Public Key
    [Documentation]    Set modifed partial public key of commitDkg
    [Arguments]    ${node}    ${partial_pubKey}
    ${node} =    To Checksum Address    ${node}
    ${result} =    Contract Function Transact    ${PROXY_CONTRACT}    setModifiedPartialPublicKey
    ...    ${node}    ${partial_pubKey}
    Log    Set node ${node} partial public key: ${partial_pubKey}

Set Modified Disqualified Nodes
    [Documentation]    Set modifed disqualified nodes of commitDkg
    [Arguments]    ${node}    ${disqualified_nodes}
    ${node} =    To Checksum Address    ${node}
    ${result} =    Contract Function Transact    ${PROXY_CONTRACT}    setModifiedDisqualifiedNodes
    ...    ${node}    ${disqualified_nodes}
    Log    Set node ${node} disqualified nodes: ${disqualified_nodes[0]}

Get Node
    [Documentation]    Get node info by node address
    [Arguments]    ${node}
    ${node} =    To Checksum Address    ${node}
    ${result} =        ${CONTROLLER_CONTRACT}    getNode    ${node}
    Log    Node info: ${result}
    RETURN    ${result}

Get Group
    [Documentation]    Get group info by group index
    [Arguments]    ${index}
    ${index} =    Convert To Integer    ${index}
    ${result} =    Contract Function Call    ${CONTROLLER_CONTRACT}    getGroup    ${index}
    Log    Group info: ${result}
    RETURN    ${result}

Is Committer
    [Documentation]    Check if the node is Committer
    [Arguments]    ${index}    ${node}
    ${group} =    Get Group    ${index}
    ${node} =    To Checksum Address    ${node}
    ${result} =    List Should Contain    ${group[5]}    ${node}
    RETURN    ${result}

Group Node Number Should Be
    [Documentation]    Check group node number
    [Arguments]    ${index}    ${number}
    ${index} =    Convert To Integer    ${index}
    ${number} =    Convert To Integer    ${number}
    ${result} =    Get Group    ${index}
    Should Be Equal   ${result[2]}    ${number}

Get Member
    [Documentation]    Get member of a given group
    [Arguments]    ${group_index}    ${member_index}
    ${group_index} =    Convert To Integer    ${group_index}
    ${member_index} =    Convert To Integer    ${member_index}
    ${result} =    Contract Function Call    ${CONTROLLER_CONTRACT}    getMember    ${group_index}    ${member_index}
    Log    The ${member_index} member of the ${group_index} group : ${result}
    RETURN    ${result}

Get Coordinator
    [Documentation]    Get coordinator of a given group
    [Arguments]    ${index}
    ${index} =    Convert To Integer    ${index}
    ${result} =    Contract Function Call    ${CONTROLLER_CONTRACT}    getCoordinator    ${index}
    Log    Coordinator of Group ${index} : ${result}
    RETURN    ${result}

Get Coordinator Instance
    [Documentation]    Get coordinator instance of a given group
    [Arguments]    ${index}
    ${address} =    Get Coordinator    ${index}
    ${coordinator_contract} =    Get Contract    ${PROXY_OUTPUT}Coordinator.sol/Coordinator.json    ${address}
    Set Global Variable    $COORDINATOR_CONTRACT   ${coordinator_contract}
    RETURN    ${coordinator_contract}

Deploy Coordinator To Test
    [Documentation]    Manualy deploy the coordinator contract to test, because the coordinator can't not be triggered without nodes
    ${contract_output} =    Run    cd contracts/; forge create src/Coordinator.sol:Coordinator --private-key ${PROXY_CONTRACT_DEPLOYER_KEY} --constructor-args 3 10
    Log    ${contract_output}
    ${lines} =	Get Lines Containing String    ${contract_output}    Deployed to: 
    ${address} =    Fetch From Right    ${lines}    Deployed to: 
    ${address} =    Strip String    ${address}
    Length Should Be    ${address}    42
    ${coordinator_contract} =    Get Contract    ${PROXY_OUTPUT}Coordinator.sol/Coordinator.json    ${address}
    Set Global Variable    $COORDINATOR_CONTRACT   ${coordinator_contract}

Get Shares
    [Documentation]    Get Shares
    [Arguments]    ${coordinator_contract}
    ${result} =    Contract Function Call    ${coordinator_contract}    getShares
    Log    Shares: ${result}
    RETURN    ${result}

Get Justifications
    [Documentation]    Get justifications
    [Arguments]    ${coordinator_contract}
    ${result} =    Contract Function Call    ${coordinator_contract}    getJustifications
    Log    Justifications: ${result}
    RETURN    ${result}

Get Participants
    [Documentation]    Get Participants
    [Arguments]    ${coordinator_contract}
    ${result} =    Contract Function Call    ${coordinator_contract}    getParticipants
    Log    Participants: ${result}
    RETURN    ${result}

Get DkgKeys
    [Documentation]    Get DkgKeys
    [Arguments]    ${coordinator_contract}
    ${result} =    Contract Function Call    ${coordinator_contract}    getDkgKeys
    Log    DkgKeys: ${result}
    RETURN    ${result}

In Phase
    [Documentation]    get Phase
    [Arguments]    ${coordinator_contract}
    ${result} =    Contract Function Call    ${coordinator_contract}    inPhase
    Log    Phase: ${result}
    RETURN    ${result}

Get Delegation Reward
    [Documentation]    Get delegation reward
    [Arguments]    ${address}
    ${address} =    To Checksum Address    ${address}
    ${result} =    Contract Function Call    ${STAKE_CONTRACT}    getDelegationReward    ${address}
    RETURN    ${result}

Get Stake
    [Documentation]    Gets the Staking amount of a given node
    [Arguments]    ${address}
    ${address} =    To Checksum Address    ${address}
    ${result} =    Contract Function Call    ${STAKE_CONTRACT}    getStake    ${address}
    RETURN    ${result}

Get Subscription
    [Documentation]    Gets the Subscription info of an subId
    [Arguments]    ${subId}
    ${subId} =    Convert To Integer    ${subId}
    ${result} =    Contract Function Call    ${ADAPTER_CONTRACT}    getSubscription    ${subId}
    RETURN    ${result}

Get Total Stake Amount
    [Documentation]    Gets total stake amount
    ${result} =    Contract Function Call    ${STAKE_CONTRACT}    getTotalStakedAmount
    RETURN    ${result}

Get Reward Rate
    [Documentation]    Gets reward rate
    ${result} =    Contract Function Call    ${STAKE_CONTRACT}    getRewardRate
    RETURN    ${result}

Get Base Reward
    [Documentation]    Gets base reward
    [Arguments]    ${address}
    ${address} =    To Checksum Address    ${address}
    ${result} =    Contract Function Call    ${STAKE_CONTRACT}    getBaseReward    ${address}
    RETURN    ${result}

Get Frozen Principal
    [Documentation]    Gets frozen principal
    [Arguments]    ${address}
    ${address} =    To Checksum Address    ${address}
    ${result} =    Contract Function Call    ${STAKE_CONTRACT}    getFrozenPrincipal    ${address}
    RETURN    ${result}

Start Stake
    [Documentation]    Start stake
    ${total_amount} =    Get Value From Env    REWARD_AMOUNT
    ${total_amount} =    Convert To Integer    ${total_amount}
    ${stake_address} =    Get Value From Env    STAKING_ADDRESS
    ${key} =    Get Value From Env    ADMIN_PRIVATE_KEY
    ${result} =    Run    cast send ${stake_address} "start(uint256 amount, uint256 rewardDuration)" ${total_amount} 2592000 --private-key=${key}

Get Community Stake Count
    [Documentation]    Gets community stake count
    ${result} =    Contract Function Call    ${STAKE_CONTRACT}    getTotalCommunityStakedAmount
    RETURN    ${result}

Claim Reward
    [Documentation]    Claim reward
    [Arguments]    ${index}
    ${address} =    Get Value From Env    STAKING_ADDRESS
    ${key} =    Get Private Key By Index    ${index}
    ${result} =    Run    cast send ${address} "claimReward()" --private-key=${key}

Claim
    [Documentation]    Claim
    [Arguments]    ${index}
    ${address} =    Get Value From Env    STAKING_ADDRESS
    ${key} =    Get Private Key By Index    ${index}
    ${result} =    Run    cast send ${address} "claim()" --private-key=${key}

Claim Frozen Principal
    [Documentation]    Claim frozen principal
    [Arguments]    ${index}
    ${address} =    Get Value From Env    STAKING_ADDRESS
    ${key} =    Get Private Key By Index    ${index}
    ${result} =    Run    cast send ${address} "claimFrozenPrincipal()" --private-key=${key}

Node Quit
    [Documentation]    Node quit
    [Arguments]    ${index}
    ${key} =    Get Private Key By Index    ${index}
    ${result} =    Run    cast send ${PROXY_CONTRACT_ADDRESS} "nodeQuit()" --private-key=${key}