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
${NODE_REGISTRY_CONTRACT}
${STAKE_CONTRACT}
${ADAPTER_CONTRACT}
${USER_CONTRACT_ADDRESS}                 NULL
${USER_CONTRACT}
${CONTRACT_ADDRESSES}                    NULL
# Test value:
${MODIFIRD_PUB_KEY}                      0x137bde2a3eca9e26d5023c8a31c7db75db47b4d1776efc144bc9cfa36403125510292172c806e0d9dd29958c8b359ea9c693179c505558cc95ca8ce6a690eb800652ce1fadb1895c06e5f28e871d8e3797f749941108195d2106a782464a09ed23ece01e5c6512317cd413fbecc36032ab7ba45f62704e9808ec2b6a2dd03d8c
${MODIFIRD_PARTIAL_PUB_KEY}              0x02
@{MODIFIRD_DISQUALIFIED_NODES}           0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC

*** Keywords ***

Set Enviorment And Deploy Contract
    [Documentation]    Set enviorment and deploy proxy contract
    Clear Log
    Kill Previous Node    20
    Clear Database
    Set Global Variable    $NODE_PROCESS_LIST    ${EMPTY_LIST}
    Setup Chain Environment
    ${result} =    Execute Script    ControllerScenarioTest.s.sol:ControllerScenarioTest    ${EMPTY}
    ${contract_addresses} =    Get Contract Address From Json    contracts/deployment-addresses.json
    Set Global Variable    $CONTRACT_ADDRESSES    ${contract_addresses}
    Set Environment Variable    ARPA_ADDRESS    ${CONTRACT_ADDRESSES['L1']['Arpa']}
    Set Environment Variable    CONTROLLER_ADDRESS    ${CONTRACT_ADDRESSES['L1']['ControllerProxy']}
    Set Environment Variable    STAKING_ADDRESS    ${CONTRACT_ADDRESSES['L1']['Staking']}
    Set Environment Variable   ADAPTER_ADDRESS    ${CONTRACT_ADDRESSES['L1']['Adapter']}
    ${result} =    Execute Script    StakeOperatorScenarioTest.s.sol:StakeOperatorScenarioTestScript    ${EMPTY}
    Bound Contract ABI
    Set ARPA Address
    Parse Node
    ${result} =    Create Node Config    ${CONTRACT_ADDRESSES['L1']['ControllerProxy']}    ${CONTRACT_ADDRESSES['L1']['Adapter']}    ${CONTRACT_ADDRESSES['L1']['ControllerRelayer']}    ${CONTRACT_ADDRESSES['L1']['Arpa']}    31337    []
    Clear Database

Init Envioirmnet
    [Documentation]    Init enviorment
    Build Contract

Build Contract
    [Documentation]    Build contract
    ${result} =    Run    cd contracts; forge build -o out; cd ..
    RETURN    ${result}

Bound Contract ABI
    [Documentation]    Bound contract ABI to handle contract function call
    ${proxy_address} =    Set Variable    ${CONTRACT_ADDRESSES['L1']['ControllerProxy']}
    ${proxy_address} =    To Checksum Address    ${proxy_address}
    Set Global Variable    $PROXY_CONTRACT_ADDRESS    ${proxy_address}
    ${proxy_contract} =    Get Contract    ${PROXY_OUTPUT}ControllerProxy.sol/ControllerProxy.json    ${PROXY_CONTRACT_ADDRESS}
    Set Global Variable    $PROXY_CONTRACT   ${proxy_contract}
    ${controller_contract} =    Get Contract    ${PROXY_OUTPUT}Controller.sol/Controller.json    ${PROXY_CONTRACT_ADDRESS}
    Set Global Variable    $CONTROLLER_CONTRACT   ${controller_contract}
    ${node_registry_address} =    Get Contract    ${PROXY_OUTPUT}NodeRegistry.sol/NodeRegistry.json    ${CONTRACT_ADDRESSES['L1']['NodeRegistry']}
    Set Global Variable    $NODE_REGISTRY_CONTRACT   ${node_registry_address}
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
    ${contract_output} =    Execute Script    GetRandomNumberScenarioTest.s.sol:GetRandomNumberScenarioTestScript    ${EMPTY}
    ${contract_address} =    Get Contract Address From File    contracts/broadcast/GetRandomNumberScenarioTest.s.sol/31337/run-latest.json
    Set Global Variable    $USER_CONTRACT_ADDRESS    ${contract_address['GetRandomNumberExample']}
    ${user_contract} =    Get Contract    ${PROXY_OUTPUT}GetRandomNumberExample.sol/GetRandomNumberExample.json    ${USER_CONTRACT_ADDRESS}
    Set Global Variable    $USER_CONTRACT   ${user_contract}

Request Randomness
    [Documentation]    Request randomness
    ${base_fee} =    Get Base Fee    ${EMPTY}
    ${base_fee} =    Convert To Integer    ${base_fee}
    ${key} =    Get Value From Env    USER_PRIVATE_KEY
    #${result} =    Contract Function Transact    ${USER_CONTRACT}    getRandomNumber    ${base_fee}
    ${result} =    Cast Send   ${USER_CONTRACT_ADDRESS}    "getRandomNumber()"    ${key}    ${EMPTY}
    Log    ${result}
    RETURN    ${result}

Check Randomness
    [Documentation]    Check the randomness
    ${last_output} =    Run    cast call ${USER_CONTRACT_ADDRESS} "lastRandomnessResult()(uint256)"
    Log    ${last_output}
    RETURN    ${last_output}

Set ARPA Address
    Set Global Variable    $ARPA_CONTRACT_ADDRESS    ${CONTRACT_ADDRESSES['L1']['Arpa']}

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
    ${base_fee} =    Get Base Fee    ${EMPTY}
    ${max_fee} =    Convert To Integer    ${base_fee / 5}
    ${result} =    Contract Function Transact    ${PROXY_CONTRACT}    setModifiedPublicKey    ${max_fee}
    ...    ${node}    ${pub_key}
    Sleep    1s
    ${result} =    Contract Function Call    ${PROXY_CONTRACT}    getModifiedDkgData    ${node}
    Log    Set node ${node} public key: ${pub_key}

Set Modified Partial Public Key
    [Documentation]    Set modifed partial public key of commitDkg
    [Arguments]    ${node}    ${partial_pubKey}
    ${node} =    To Checksum Address    ${node}
    ${base_fee} =    Get Base Fee    ${EMPTY}
    ${max_fee} =    Convert To Integer    ${base_fee / 5}
    ${result} =    Contract Function Transact    ${PROXY_CONTRACT}    setModifiedPartialPublicKey    ${max_fee}
    ...    ${node}    ${partial_pubKey}
    Log    Set node ${node} partial public key: ${partial_pubKey}

Set Modified Disqualified Nodes
    [Documentation]    Set modifed disqualified nodes of commitDkg
    [Arguments]    ${node}    ${disqualified_nodes}
    ${node} =    To Checksum Address    ${node}
    ${base_fee} =    Get Base Fee    ${EMPTY}
    ${max_fee} =    Convert To Integer    ${base_fee}
    ${result} =    Contract Function Transact    ${PROXY_CONTRACT}    setModifiedDisqualifiedNodes    ${max_fee}
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
    ${result} =    Cast Send    ${stake_address}    "start(uint256 amount, uint256 rewardDuration)" ${total_amount} 2592000    ${key}    ${EMPTY}

Get Community Stake Count
    [Documentation]    Gets community stake count
    ${result} =    Contract Function Call    ${STAKE_CONTRACT}    getTotalCommunityStakedAmount
    RETURN    ${result}

Claim Reward
    [Documentation]    Claim reward
    [Arguments]    ${index}
    ${address} =    Get Value From Env    STAKING_ADDRESS
    ${key} =    Get Private Key By Index    ${index}
    ${result} =    Cast Send    ${address}    "claimReward()"    ${key}    ${EMPTY}

Claim
    [Documentation]    Claim
    [Arguments]    ${index}
    ${address} =    Get Value From Env    STAKING_ADDRESS
    ${key} =    Get Private Key By Index    ${index}
    ${result} =    Cast Send    ${address}    "claim()"    ${key}    ${EMPTY}

Claim Frozen Principal
    [Documentation]    Claim frozen principal
    [Arguments]    ${index}
    ${address} =    Get Value From Env    STAKING_ADDRESS
    ${key} =    Get Private Key By Index    ${index}
    ${result} =    Cast Send    ${address}    "claimFrozenPrincipal()"    ${key}    ${EMPTY}

Node Quit
    [Documentation]    Node quit
    [Arguments]    ${index}
    ${key} =    Get Private Key By Index    ${index}
    ${result} =    Cast Send    ${CONTRACT_ADDRESSES['L1']['NodeRegistry']}    "nodeQuit()"    ${key}    ${EMPTY}

Execute Script
    [Arguments]    ${path}    ${url}   
    ${base_fee} =    Get Base Fee    ${url}
    ${max_fee} =    Convert To Integer    ${base_fee}
    Exec Script    ${path}    ${url}    ${max_fee}

