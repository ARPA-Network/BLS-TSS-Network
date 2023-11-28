"""
Node management functions for the Arpa test environment.
"""

import os
import platform
import subprocess
import sys
import re
import grpc
sys.path.insert(1, 'tests/scenarios/src/environment/proto')
import management_pb2
import management_pb2_grpc
from google.protobuf.empty_pb2 import Empty



def get_request(request_name, **args):
    """
    Return an instance of the given Request object based on the input.
    request_name: name of the request.
    args: dictionary of arguments to be set in the request object.
    """
    try:
        request = getattr(management_pb2, request_name + "Request")
        request_object = request()
        if args:
            for key, value in args.items():
                setattr(request_object, key, value)
        return request_object
    except AttributeError:
        return Empty()

def get_reply(reply_name):
    """
    Return an instance of the given Reply object.
    reply_name: name of the reply.
    """
    try:
        reply = getattr(management_pb2, reply_name + "Reply")
        return reply()
    except AttributeError:
        return Empty()

def call_request(endpoint, request_name, **args):
    """
    Call the given request and return the response.
    endpoint: endpoint of the node.
    request_name: name of the request.
    args: dictionary of arguments to be set in the request object.
    """
    channel = grpc.insecure_channel(endpoint)
    stub = management_pb2_grpc.ManagementServiceStub(channel)
    request = get_request(request_name, **args)
    function = getattr(stub, request_name)
    metadata = []
    metadata.append(('authorization', 'for_test'))
    response = function(request, metadata=metadata)
    return response

class Account:
    """
    Class of a account to manage the node.
    """
    def __init__(self, address, key):
        self.address = address
        self.key = key

def parse_chain_result_to_account_list():
    """
    Parse the result of the chain command to a list of Account objects.
    """
    with open('tests/scenarios/src/environment/node_config/accounts.txt', 'r',
              encoding='utf-8') as file:
        # creating two arrays of accounts and private keys
        accounts = []
        private_keys = []
        # looping over lines in the opened file
        for line in file:
            is_private_key = False
            # locating private keys
            matches = re.finditer('(0x)?[A-Fa-f0-9]{64}', line)
            for match in matches:
                private_keys.append(match.group())
                is_private_key = True
            if is_private_key:
                continue
            # locating accounts
            matches = re.finditer('(0x)?[A-Fa-f0-9]{40}', line)
            for match in matches:
                accounts.append(match.group())
        # Create a list of Account objects using the namedtuple syntax
        account_list = [Account(a,k) for a,k in zip(accounts, private_keys)]
        return account_list

def create_node_config(controller_address, adapter_address):
    """
    Create the node config files.
    """
    # Get dictionary of account/private key pairs
    account_list = parse_chain_result_to_account_list()
    # Loop through accounts
    i = 0
    for account in account_list:
        key = account.key.replace('0x', '')
        # Create filename
        file_name = f'config{i + 1}.yml'
        # Create contents
        content = f"""node_committer_rpc_endpoint: \"0.0.0.0:501{61 + i}\"

node_management_rpc_endpoint: \"0.0.0.0:50{201 + i}\"

node_management_rpc_token: "for_test"

provider_endpoint: "http://127.0.0.1:8545"

chain_id: 31337

controller_address: "{controller_address}"
adapter_address: "{adapter_address}"

data_path: "./data{i + 1}.sqlite"

account:
  private_key: \"{key}\"

listeners:
  - l_type: Block
    interval_millis: 0
    use_jitter: true
  - l_type: NewRandomnessTask
    interval_millis: 0
    use_jitter: true
  - l_type: PreGrouping
    interval_millis: 0
    use_jitter: true
  - l_type: PostCommitGrouping
    interval_millis: 1000
    use_jitter: false
  - l_type: PostGrouping
    interval_millis: 1000
    use_jitter: false
  - l_type: ReadyToHandleRandomnessTask
    interval_millis: 1000
    use_jitter: false
  - l_type: RandomnessSignatureAggregation
    interval_millis: 1000
    use_jitter: false

time_limits:
  dkg_timeout_duration: 40
  randomness_task_exclusive_window: 10
  listener_interval_millis: 1000
  dkg_wait_for_phase_interval_millis: 1000
  provider_polling_interval_millis: 1000
  provider_reset_descriptor:
    interval_millis: 5000
    max_attempts: 17280
    use_jitter: false
  contract_transaction_retry_descriptor:
    base: 2
    factor: 1000
    max_attempts: 3
    use_jitter: true
  contract_view_retry_descriptor:
    base: 2
    factor: 500
    max_attempts: 5
    use_jitter: true
  commit_partial_signature_retry_descriptor:
    base: 2
    factor: 1000
    max_attempts: 5
    use_jitter: false

context_logging: false
node_id: {i + 1}
"""
        # Write out to file
        with open('tests/scenarios/src/environment/node_config/'+file_name, 'w',
                  encoding='utf-8') as file:
            file.write(content)
        i += 1


def start_node(node_idx):
    """
    Start a node.
    node_idx: index of the node.
    """
    root_path = os.path.dirname(os.path.abspath(__file__))
    root_path = root_path.split("/tests/")[0]
    cmd = ("cd crates/arpa-node;"
       "cargo run --bin node-client -- -c "
       "{}/tests/scenarios/src/environment/node_config/config{}.yml"
      ).format(root_path, node_idx)
    log_path = f"crates/arpa-node/log/running/node{node_idx}.log"
    # Check if file exists, if not create an empty file
    if not os.path.exists(log_path):
        open(log_path, 'w', encoding='UTF-8').close()
    with open(log_path, 'w', encoding='utf-8') as log_file:
        proc = subprocess.Popen(cmd, shell=True, stdout=log_file,
                                stderr=subprocess.STDOUT, cwd=root_path)
    return proc

def kill_node(proc):
    """
    Kill a node.
    proc: process of the node.
    """
    proc.kill()
    proc.terminate()

def kill_process_by_port(port):
    """
    Kill process by port.
    port: port number.
    """
    if platform.system() == "Windows":
        command = f'FOR /F "tokens=5 delims= " %P IN (\'netstat -a -n -o ^| findstr :{port}\') DO TaskKill.exe /F /PID %P'
        os.system(command)
    else:
        command = f'lsof -ti :{port} | xargs kill -9'
        subprocess.call(command, shell=True)

def kill_node_by_index(index):
    """
    Kill a node by its index.
    index: index of the node.
    """
    client_port = 50161 + int(index) - 1
    committer_port = 50201 + int(index) - 1
    kill_process_by_port(client_port)
    kill_process_by_port(committer_port)


def get_node_port_from_index(node_idx):
    """
    Get the port of a node from its index.
    node_idx: index of the node.
    """
    port = 50201 + int(node_idx) - 1
    return  'localhost:' + str(port)

def add_process_to_list(proc, node_list):
    """
    Add a process to the list of processes.
    proc: process of the node.
    node_list: list of processes.
    """
    if len(node_list) == 0:
        node_list = []
    node_list.append(proc)
    return node_list

def kill_previous_node(node_number=10):
    """
    Kill all previous processes.
    """
    for i in range(1, node_number + 1):
        kill_node_by_index(i)

def print_node_process():
    """
    Get the process of a node.
    """
    # Find all processes using ports 50061-50110
    cmd = 'lsof -i :50061-50220'
    proc = subprocess.Popen(cmd.split(), stdout=subprocess.PIPE)
    out,_ = proc.communicate()
    for line in out.splitlines()[1:]:
        fields = line.strip().split()
        if fields:
            if fields[0] == b'node-clie':
                print(fields)
