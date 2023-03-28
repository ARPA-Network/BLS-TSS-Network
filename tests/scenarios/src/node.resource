*** Settings ***
Documentation       This resource file use to interact with nodes
Library             Process
Library             OperatingSystem
Library             environment/node.py
Library             environment/log.py
Resource            common.resource

*** Variables ***
${PROTO_PATH}                       tests/scenarios/src/environment/proto
${CONFIG_PATH}                      tests/scenarios/src/environment/node_config/
#Test variables
${NODES_PORT}                       localhost:50101
*** Keywords ***
Compile Proto
    Run    mv ${PROTO_PATH}; mkdir ${PROTO_PATH}
    Run    cp crates/arpa-node/proto/*.proto ${PROTO_PATH}
    Run    cp crates/randcast-contract-mock/proto/*.proto ${PROTO_PATH}
    Run    python -m grpc_tools.protoc -I=${PROTO_PATH} --python_out=${PROTO_PATH} --grpc_python_out=${PROTO_PATH} management.proto

List Fixed Tasks
    [Arguments]    ${node_index}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${fixed_tasks} =    Call Request    ${node_port}    ListFixedTasks
    RETURN    ${fixed_tasks}

Start Listener
    [Arguments]    ${node_index}    ${task_type}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${res} =    Call Request    ${node_port}    StartListener    task_type=${task_type}
    RETURN    ${res}

Shutdown Listener
    [Arguments]    ${node_index}    ${task_type}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${res} =    Call Request    ${node_port}    ShutdownListener    task_type=${task_type}
    RETURN    ${res}


Node Register 
    [Arguments]    ${node_index}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${res} =    Call Request    ${node_port}    NodeRegister
    RETURN    ${res}

Activate Node
    [Arguments]    ${node_index}
    ${node_port} =  Get Node Port From Index    ${node_index}
    ${res} =    Call Request    ${node_port}    NodeActivate
    RETURN    ${res}

Node Quit
    [Arguments]    ${node_index}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${res} =    Call Request    ${node_port}    NodeQuit
    RETURN    ${res}

Shutdown Node
    [Arguments]    ${node_index}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${res} =    Call Request    ${node_port}    ShutdownNode
    RETURN    ${res}

Get Node Info
    [Arguments]    ${node_index}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${node_info} =    Call Request    ${node_port}    GetNodeInfo
    RETURN    ${node_info}

Get Group Info
    [Arguments]    ${node_index}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${group_info} =    Call Request    ${node_port}    GetGroupInfo
    RETURN    ${group_info}

Post Process Dkg
    [Arguments]    ${node_index}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${res} =    Call Request    ${node_port}    PostProcessDkg
    RETURN    ${res}

Partial Sign
    [Arguments]    ${node_index}    ${sig_index}    ${threshold}    ${msg}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${msg} =    Convert To Bytes    ${msg}
    ${res} =    Call Request    ${node_port}    PartialSign    sig_index=${sig_index}    threshold=${threshold}    msg=${msg}
    RETURN    ${res}

Aggregate Partial Signatures
    [Arguments]    ${node_index}    ${threshold}    ${partial_sigs}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${msg} =    Convert To Bytes    ${partial_sigs}
    ${res} =    Call Request    ${node_port}    AggregatePartialSignatures    threshold=${threshold}    partial_sigs=${partial_sigs}
    RETURN    ${res}

Verify Sig
    [Arguments]    ${node_index}    ${public}    ${msg}    ${sig}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${public} =    Convert To Bytes    ${public}
    ${msg} =    Convert To Bytes    ${msg}
    ${sig} =    Convert To Bytes    ${sig}
    ${res} =    Call Request    ${node_port}    VerifySig    public=${public}    msg=${msg}    sig=${sig}
    RETURN    ${res}

Verify Partial Sigs
    [Arguments]    ${node_index}    ${publics}    ${msg}    ${partial_sigs}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${public} =    Convert To Bytes    ${publics}
    ${msg} =    Convert To Bytes    ${msg}
    ${partial_sigs} =    Convert To Bytes    ${partial_sigs}
    ${res} =    Call Request    ${node_port}    VerifyPartialSigs    publics=${public}    msg=${msg}    partial_sigs=${partial_sigs}
    RETURN    ${res}

Send Partial Sig
    [Arguments]    ${node_index}    ${member_id_address}    ${msg}    ${sig_index}    ${partial_sig}
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${msg} =    Convert To Bytes    ${msg}
    ${partial_sig} =    Convert To Bytes    ${partial_sig}
    ${res} =    Call Request    ${node_port}    SendPartialSig    member_id_address=${member_id_address}
    ...    msg=${msg}    sig_index=${sig_index}    partial_sig=${partial_sig}
    RETURN    ${res}

Fulfill Randomness
    [Arguments]    ${node_index}    ${group_index}    ${sig_index}    ${sig}    ${partial_sigs}
    [Documentation]    partial_sigs need to be inited by Create Dictionary 
    ...    #@{my_dict}=    Create Dictionary    key1=val1    key2=val2    key3=val3
    ${node_port} =    Get Node Port From Index    ${node_index}
    ${sig} =    Convert To Bytes    ${sig}
    ${res} =    Call Request    ${node_port}    FulfillRandomness    group_index=${group_index}    sig_index=${sig_index}
    ...    sig=${sig}    partial_sigs=${partial_sigs}
    RETURN    ${res}

Get Address By Index
    [Arguments]    ${node_index}
    RETURN    ${NODE_ACCOUNT_LIST[${node_index} - 1].address}

Add ARPA
    [Documentation]    Test Add ARPA
    [Arguments]    ${address}    ${amount}
    ${from} =    Get Address By Index    20
    ${result} =    Run    cast send ${ARPA_CONTRACT_ADDRESS} "mint(address to, uint256 amount)" ${address} ${amount} --from=${from}

Add Balance And Run Node
    [Arguments]    ${node_index}
    ${address} =    Get Address By Index    ${node_index}
    Add ARPA    ${address}    100000
    ${proc} =    Start Node    ${node_index}
    ${register_result} =    Get Keyword From Log    ${proc}    Transaction successful(node_register)    200
    Should Not Be Empty    ${register_result}
    ${list} =    Add Process To List    ${proc}    ${NODE_PROCESS_LIST}
    Set Global Variable    $NODE_PROCESS_LIST    ${list}
    RETURN    ${proc}