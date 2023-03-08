*** Settings ***
Documentation       This resource file sets the constants and variables
...                 that applies to all test suites and test cases

Library             Process
Library             String
Library             OperatingSystem
Library             environment/local_ethereum.py
Library             environment/node.py

Resource            common.resource

*** Variables ***
# Proxy contract:
${PROXY_CONTRACT_PATH}                   src/Proxy.sol:Proxy
${PROXY_CONTRACT_DEPLOYER_KEY}           0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
${PROXY_CONTRACT_ADDRESS}
${PROXY_OUTPUT}                          out/
${PROXY_CONTRACT}
${CONTROLLER_CONTRACT}
${COORDINATOR_CONTRACT}
${COORDINATOR_CONTRACT}

# Test value:
${TEST_NODE_ADDRESS}                     0x70997970c51812dc3a010c7d01b50e0d17dc79c8
${TEST_PRIVATE_KEY}                      0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
${MODIFIRD_PUB_KEY}                      0x01
${MODIFIRD_PARTIAL_PUB_KEY}              0x02
@{MODIFIRD_DISQUALIFIED_NODES}           0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC

*** Keywords ***
Set Enviorment And Deploy Contract
    [Documentation]    Set enviorment and deploy proxy contract
    Setup Scenario Testing Environment
    Deploy Controller
    Deploy Proxy Contract
    Set ARPA Address
    Create Node Config    ${PROXY_CONTRACT_ADDRESS}

Deploy Proxy Contract
    [Documentation]    Deploy the proxy contract, the proxy and controller share the same address
    ${contract_output} =    Run    cd contracts/; forge create ${PROXY_CONTRACT_PATH} --constructor-args 0x9fE46736679d2D9a65F0992F2272dE9f3c7fa6e0 --private-key ${PROXY_CONTRACT_DEPLOYER_KEY} -o ${PROXY_OUTPUT} 
    Log    ${contract_output}
    ${lines} =	Get Lines Containing String    ${contract_output}    Deployed to: 
    ${address} =    Fetch From Right    ${lines}    Deployed to: 
    ${address} =    Strip String    ${address}
    Length Should Be    ${address}    42
    Set Global Variable    $PROXY_CONTRACT_ADDRESS    ${address}
    ${proxy_contract} =    Get Contract    contracts/${PROXY_OUTPUT}Proxy.sol/Proxy.json    ${address}
    Set Global Variable    $PROXY_CONTRACT   ${proxy_contract}
    ${controller_contract} =    Get Contract    contracts/${PROXY_OUTPUT}Controller.sol/Controller.json    ${address}
    Set Global Variable    $CONTROLLER_CONTRACT   ${controller_contract}

Deploy User Contract And Request Randomness
    [Documentation]    Deploy the user contract
    ${contract_output} =    Run    cd contracts/; forge script script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url http://localhost:8545 --broadcast

Check Randomness
    [Documentation]    Check the randomness
    ${user_contract} =    Get Value From Env    USER_ADDRESS
    #${user_contract_output} =    Run    cast call ${PROXY_CONTRACT_ADDRESS} "lastOutput()returns(uint256)"
    ${adapter_contract_output} =    Run    cast call ${user_contract} "lastRandomnessResult()"
    Log    ${adapter_contract_output}

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
    Should Be Equal   ${result[3]}    ${number}

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
    ${coordinator_contract} =    Get Contract    contracts/${PROXY_OUTPUT}Coordinator.sol/Coordinator.json    ${address}
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
    ${coordinator_contract} =    Get Contract    contracts/${PROXY_OUTPUT}Coordinator.sol/Coordinator.json    ${address}
    Set Global Variable    $COORDINATOR_CONTRACT   ${coordinator_contract}

Get Shares
    [Documentation]    Get Shares
    ${result} =    Contract Function Call    ${COORDINATOR_CONTRACT}    getShares
    Log    Shares: ${result}
    RETURN    ${result}

Get Justifications
    [Documentation]    Get justifications
    ${result} =    Contract Function Call    ${COORDINATOR_CONTRACT}    getJustifications
    Log    Justifications: ${result}
    RETURN    ${result}

Get Participants
    [Documentation]    Get Participants
    ${result} =    Contract Function Call    ${COORDINATOR_CONTRACT}    getParticipants
    Log    Participants: ${result}
    RETURN    ${result}

Get DkgKeys
    [Documentation]    Get DkgKeys
    ${result} =    Contract Function Call    ${COORDINATOR_CONTRACT}    getDkgKeys
    Log    DkgKeys: ${result}
    RETURN    ${result}

In Phase
    [Documentation]    get Phase
    ${result} =    Contract Function Call    ${COORDINATOR_CONTRACT}    inPhase
    Log    Phase: ${result}
    RETURN    ${result}

Request Randomness
    [Documentation]    Request randomness
    ${result} =    Run   forge script script/GetRandomNumberLocalTest.s.sol:GetRandomNumberLocalTestScript --fork-url http://localhost:8545 --broadcast
    Log    Request randomness: ${result}

Get Test
    [Documentation]    Get test
    [Arguments]    ${index}
    ${index} =    Convert To Integer    ${index}
    ${result} =    Contract Function Call    ${CONTROLLER_CONTRACT}    test    ${index}
    Log    Test: ${result}
    RETURN    ${result}