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
${PROXY_CONTRACT_PATH}                   src/Proxy.sol:Proxy
${PROXY_CONTRACT_DEPLOYER_KEY}           0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
${PROXY_CONTRACT_ADDRESS}
${PROXY_OUTPUT}                          contracts/out/
${PROXY_CONTRACT}
${CONTROLLER_CONTRACT}
${STAKE_CONTRACT}
${DEPLOY_CONTROLLER_SCRIPT}              DeployControllerLocalTest.s.sol:DeployControllerTestScript
${PROXY_CONFIG_SCRIPT}                   ProxyConfigLocalTest.s.sol:ProxyConfigLocalTestScript
${REQUEST_RANDOMNESS_SCRIPT}             GetRandomNumberProxy.s.sol:GetRandomNumberProxyScript
${USER_CONTRACT_ADDRESS}
${USER_CONTRACT}
# Test value:
${TEST_NODE_ADDRESS}                     0x70997970c51812dc3a010c7d01b50e0d17dc79c8
${TEST_PRIVATE_KEY}                      0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
${MODIFIRD_PUB_KEY}                      0x01
${MODIFIRD_PARTIAL_PUB_KEY}              0x02
@{MODIFIRD_DISQUALIFIED_NODES}           0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC

*** Keywords ***
Set Enviorment And Deploy Contract
    [Documentation]    Set enviorment and deploy proxy contract
    Clear Log
    Setup Chain Environment
    ${result} =    Run    cd contracts/;forge build -o out/ --revert-strings debug
    ${result} =    Exec Script    ControllerLocalTestProxy.s.sol:ControllerLocalTestProxyScript
    #${result} =    Exec Script    ControllerLocalTest.s.sol:ControllerLocalTestScript
    ${result} =    Exec Script    StakeOperatorLocalTest.s.sol:StakeOperatorLocalTestScript
    Bound Contract ABI
    Set ARPA Address
    ${adapter_address} =    Get Value From Env    ADAPTER_ADDRESS
    ${result} =    Create Node Config    ${PROXY_CONTRACT_ADDRESS}    ${adapter_address}
    Kill Previous Node
    Set Global Variable    $NODE_PROCESS_LIST    ${EMPTY_LIST}

Bound Contract ABI
    [Documentation]    Bound contract ABI to handle contract function call
    ${proxy_address} =    Get Value From Env    PROXY_ADDRESS
    ${proxy_address} =    To Checksum Address    ${proxy_address}
    Set Global Variable    $PROXY_CONTRACT_ADDRESS    ${proxy_address}
    ${proxy_contract} =    Get Contract    ${PROXY_OUTPUT}Proxy.sol/Proxy.json    ${PROXY_CONTRACT_ADDRESS}
    Set Global Variable    $PROXY_CONTRACT   ${proxy_contract}
    ${controller_contract} =    Get Contract    ${PROXY_OUTPUT}Controller.sol/Controller.json    ${PROXY_CONTRACT_ADDRESS}
    Set Global Variable    $CONTROLLER_CONTRACT   ${controller_contract}
    ${stake_address} =    Get Value From Env    STAKING_ADDRESS
    ${stake_address} =    To Checksum Address    ${stake_address}
    ${stake_contract} =    Get Contract    ${PROXY_OUTPUT}IStaking.sol/IStaking.json    ${stake_address}
    Set Global Variable    $STAKE_CONTRACT   ${stake_contract}

Deploy User Contract And Request Randomness
    [Documentation]    Deploy the user contract
    ${user_contract_address} =    Set Variable    ${USER_CONTRACT_ADDRESS}
    ${user_contract} =    Get Contract    ${PROXY_OUTPUT}GetRandomNumberExample.sol/GetRandomNumberExample.json    ${user_contract_address}
    Set Global Variable    $USER_CONTRACT   ${user_contract}
    IF  ${user_contract_address == ''}
        ${contract_output} =    Exec Script    GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript
        ${contract_address} =    Get Contract Address From File    contracts/broadcast/GetRandomNumberLocalTest.s.sol/31337/run-latest.json
        Set Global Variable    $USER_CONTRACT_ADDRESS    ${contract_address[0]}
        ${result} =    All Nodes Have Keyword    received new randomness task    ${NODE_PROCESS_LIST}
    ELSE
        ${request_id} =    Contract Function Transact    ${USER_CONTRACT}    getRandomNumber
    END

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
    ${result} =    Contract Function Call    ${CONTROLLER_CONTRACT}    getNode    ${node}
    Log    Node info: ${result}
    RETURN    ${result}

Get Group
    [Documentation]    Get group info by group index
    [Arguments]    ${index}
    ${index} =    Convert To Integer    ${index}
    ${result} =    Contract Function Call    ${CONTROLLER_CONTRACT}    getGroup    ${index}
    Log    Group info: ${result}
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

Request Randomness
    [Documentation]    Request randomness
    ${result} =    Run   forge script script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url http://localhost:8545 --broadcast
    Log    Request randomness: ${result}

Get Reward
    [Documentation]    Get reward
    [Arguments]    ${address}
    ${address} =    To Checksum Address    ${address}
    ${result} =    Contract Function Call    ${CONTROLLER_CONTRACT}    getRewards    ${address}
    RETURN    ${result}

Get Node Staking
    [Documentation]    Gets the Staking amount of a given node
    [Arguments]    ${address}
    ${result} =    Contract Function Call    ${STAKE_CONTRACT}    getStake    ${address}
    RETURN    ${result}