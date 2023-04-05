"""
Node management functions for the Arpa test environment.
"""
import os
import subprocess
import sys
import re
import grpc
from google.protobuf.empty_pb2 import Empty
sys.path.insert(1, 'tests/scenarios/src/environment/proto')
import management_pb2_grpc
import management_pb2

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
            # locating accounts
            matches = re.finditer('(0x)?[A-Fa-f0-9]{40}', line)
            for m in matches:
                accounts.append(m.group())

            # locating private keys
            matches = re.finditer('(0x)?[A-Fa-f0-9]{64}', line)
            for m in matches:
                private_keys.append(m.group())
        # Create a list of Account objects using the namedtuple syntax
        account_list = [Account(a,k) for a,k in zip(accounts, private_keys)]
        return account_list

def create_node_config(controller_address):
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
        content = f"""node_committer_rpc_endpoint: \"[::1]:501{61 + i}\"

node_management_rpc_endpoint: \"[::1]:50{201 + i}\"

node_management_rpc_token: "for_test"

provider_endpoint: "http://127.0.0.1:8545"

chain_id: 31337

controller_address: "{controller_address}"

data_path: "./data{i + 1}.sqlite"

account:
  private_key: \"{key}\"

context_logging: false"""
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
       "cargo run --bin node-client -- -m new-run -c "
       "{}/tests/scenarios/src/environment/node_config/config{}.yml"
      ).format(root_path, node_idx)
    print(cmd)
    proc = subprocess.Popen(cmd, shell=True, stdout=subprocess.PIPE,
                            stderr=subprocess.STDOUT, cwd=root_path)
    return proc

def kill_node(proc):
    """
    Kill a node.
    proc: process of the node.
    """
    proc.kill()
    proc.terminate()

def get_node_port_from_index(node_idx):
    """
    Get the port of a node from its index.
    node_idx: index of the node.
    """
    port = 50161 + int(node_idx) - 1
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

def kill_previous_node():
    """
    Kill all previous processes.
    """
    # Find all processes using ports 50061-50110
    cmd = 'lsof -i :50061-50220'
    p = subprocess.Popen(cmd.split(), stdout=subprocess.PIPE)
    out, err = p.communicate()
    # Kill each process found
    for line in out.splitlines()[1:]:
        fields = line.strip().split()
        if fields:
            if fields[0] == b'node-clie':
                pid = int(fields[1])
                subprocess.call(['kill', '-9', str(pid)])

