*** Settings ***
Documentation       This resource file sets the constants and variables
...                 that applies to all test suites and test cases

Library             Process
Library             String
Library             OperatingSystem
Library             environment/local_ethereum.py

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

# Test value:
${TEST_NODE_ADDRESS}                     0x70997970c51812dc3a010c7d01b50e0d17dc79c8
${TEST_PRIVATE_KEY}                      0xac0974bec39a17e36ba4a6b4d238ff944bacb478cbed5efcae784d7bf4f2ff80
${MODIFIRD_PUB_KEY}                      0x01
${MODIFIRD_PARTIAL_PUB_KEY}              0x02
@{MODIFIRD_DISQUALIFIED_NODES}           0x3C44CdDdB6a900fa2b585dd299e03d12FA4293BC

*** Keywords ***
Deploy Proxy Contract
    [Documentation]    Deploy the proxy contract, the proxy and controller share the same address
    ${contract_output} =    Run    cd contracts/; forge create ${PROXY_CONTRACT_PATH} --private-key ${PROXY_CONTRACT_DEPLOYER_KEY} -o ${PROXY_OUTPUT} 
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

Get Modified CommitdDkg
    [Documentation]    Get all modifed value of commitDkg
    [Arguments]    ${node}
    ${node} =    To Checksum Address    ${node}
    ${result} =    Contract Function Call    ${proxy_contract}    getModifiedDkgData    ${node}
    Log    Modified dkg data: ${result}
    RETURN    ${result}

Set Modified CommitdDkg
    [Documentation]    Set all modifed value of commitDkg
    [Arguments]    ${node}
    Set Modified Public Key    ${node}    ${MODIFIRD_PUB_KEY}
    Set Modified Partial Public Key    ${node}    ${MODIFIRD_PARTIAL_PUB_KEY}
    Set Modified Disqualified Nodes    ${node}    ${MODIFIRD_DISQUALIFIED_NODES}
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

Node In Members
    [Documentation]    To check if node in members
    [Arguments]    ${index}    ${node}
    ${index} =    Convert To Integer    ${index}
    ${node} =    To Checksum Address    ${node}
    ${result} =    Contract Function Call    ${CONTROLLER_CONTRACT}    NodeInMembers    ${index}    ${node}
    Log    Node in members : ${result}
    RETURN    ${result}

Partial Key Registered
    [Documentation]    Check to see if a group has a partial public key registered for a given node
    [Arguments]    ${index}    ${node}
    ${index} =    Convert To Integer    ${index}
    ${node} =    To Checksum Address    ${node}
    ${result} =    Contract Function Call    ${CONTROLLER_CONTRACT}    PartialKeyRegistered    ${index}    ${node}
    Log    Partial public key registered : ${result}
    RETURN    ${result}

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
