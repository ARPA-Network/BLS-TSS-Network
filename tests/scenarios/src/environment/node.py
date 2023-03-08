import os
from dotenv import load_dotenv
import subprocess
import grpc
import sys
import re
from google.protobuf.empty_pb2 import Empty

sys.path.insert(1, 'tests/scenarios/src/environment/proto')
import management_pb2_grpc
import management_pb2

def get_request(request_name, **args):
    # Return an instance of the given Request object based on the input
    try:
        RequestObject = getattr(management_pb2, request_name + "Request")
        request_object = RequestObject()
        if args:
            for key, value in args.items():
                setattr(request_object, key, value)
        return request_object
    except AttributeError:
        return Empty()

def get_reply(reply_name):
    try:
        ReplyObject = getattr(management_pb2, reply_name + "Reply")
        return ReplyObject()
    except AttributeError:
        return Empty()

def call_request(endpoint, request_name, **args):
    channel = grpc.insecure_channel(endpoint)
    stub = management_pb2_grpc.ManagementServiceStub(channel)
    request = get_request(request_name, **args)
    function = getattr(stub, request_name)
    metadata = []
    metadata.append(('authorization', 'for_test'))
    response = function(request, metadata=metadata)
    return response
#call_request("localhost:50099", "NodeQuit")

class Account:
    def __init__(self, address, key):
        self.address = address 
        self.key = key

def parse_chain_result_to_account_list():
    with open('tests/scenarios/src/environment/node_config/accounts.txt', 'r') as f:
        # creating two arrays of accounts and private keys
        accounts = []
        private_keys = []
        
        # looping over lines in the opened file
        for line in f:
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
    # Get dictionary of account/private key pairs
    account_list = parse_chain_result_to_account_list()
    # Loop through accounts
    for i in range(len(account_list)):
        key = account_list[i].key.replace('0x', '')
        # Create filename
        file_name = f'config{i + 1}.yml'
        # Create contents 
        content = f"""node_committer_rpc_endpoint: \"[::1]:500{61 + i}\"

node_management_rpc_endpoint: \"[::1]:50{101 + i}\"

node_management_rpc_token: "for_test"

provider_endpoint: "http://127.0.0.1:8545"

chain_id: 31337

controller_address: "{controller_address}"

data_path: "./data{i + 1}.sqlite"

account:
  private_key: \"{key}\"

context_logging: false"""
        # Write out to file
        with open('tests/scenarios/src/environment/node_config/'+file_name, 'w') as f:
            f.write(content)

def start_node(node_idx):
    port = 50061 + int(node_idx) - 1
    os.system(f'kill $(lsof -t -i:{port})')
    root_path = os.path.dirname(os.path.abspath(__file__))
    root_path = root_path.split("/tests/")[0]
    cmd = ("cd {}/crates/arpa-node; "
       "cargo run --bin node-client -- -m new-run -c "
       "{}/tests/scenarios/src/environment/node_config/config{}.yml"
      ).format(root_path, root_path, node_idx)
    proc = subprocess.Popen(cmd, shell=True)
    return proc

def kill_node(proc):
    proc.kill()

def get_node_port_from_index(node_idx):
    port = 50061 + int(node_idx) - 1
    return  'localhost:' + str(port)

def get_value_from_env(name):
    load_dotenv("contracts/.env")
    value = os.environ.get(name)
    print(value)
    return value