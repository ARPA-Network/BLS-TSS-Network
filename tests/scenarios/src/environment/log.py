"""
This module contains functions to retrieve information from the logs
"""
import json
import os
import re
import time

def get_log_info(log, keyword):
    """
    Retrieve information for a given keyword from the log
    :param log: file-like object with the log data
    :param keyword: keyword to search for in the log
    :return: dictionary with the relevant information found for the keyword
    """

    for line in log:
        try:
            log_json = json.loads(line)
            message = log_json["message"]
            message = json.dumps(message)
            if message.find(keyword) != -1:
                return message
        except ValueError:
            continue
    return None


def get_keyword_from_node_log(node_idx, keyword, retry_time=30):
    """
    Run a command as a sub-process and check the logs for a given keyword
    :param process: sub-process object
    :param keyword: keyword to look for in the log
    :return: the relevant information found for the keyword
    """
    i =  0
    log_path = f"crates/arpa-node/log/running/node{node_idx}.log"
    while i < retry_time:
        i = i + 1
        with open(log_path, 'r', encoding='UTF-8') as process:
            log_info = get_log_info(process, keyword)
            if log_info is not None:
                return log_info
        time.sleep(1)
    return None


def have_node_got_keyword(keyword, node_process_list, retry_time=30):
    """
    Get a keyword from all nodes
    :param keyword: keyword to look for in the log
    :return: whether the relevant information was found for the keyword
    """
    while retry_time > 0:
        retry_time = retry_time - 1
        node_idx = 1
        while node_idx <= len(node_process_list):
            log_info = get_keyword_from_node_log(node_idx, keyword, 2)
            if log_info is not None:
                return True
            node_idx = node_idx + 1
    return False


def all_nodes_have_keyword(keyword, node_process_list, retry_time=30, node_idx=1):
    """
    Check if all nodes have a keyword in their logs
    :param keyword: keyword to look for in the log
    :return: dictionary with the relevant information found for the keyword
    """
    start_idx = node_idx
    while node_idx < start_idx + len(node_process_list):
        log_info = get_keyword_from_node_log(node_idx, keyword, retry_time)
        if log_info is None:
            return False
        node_idx = node_idx + 1
    clear_log()
    return True

def clear_log(path='crates/arpa-node/log/running/'):
    """
    Clear the node log file
    """
    os.makedirs(path, exist_ok=True)
    node_idx = 1
    while node_idx <= 10:
        open_path = path + 'node' + str(node_idx) + '.log'
        if os.path.exists(open_path):
            open(open_path, 'w', encoding='UTF-8').close()
        node_idx = node_idx + 1
    open(path + 'anvil-chain.log', 'w', encoding='UTF-8').close()
    open(path + 'op.log', 'w', encoding='UTF-8').close()

def clear_one_log(path):
    """
    Clear a log file
    """
    open(path, 'w', encoding='UTF-8').close()

def get_err_log_from_chain():
    """
    Get the error log from the chain
    """
    log_path = "crates/arpa-node/log/running/anvil-chain.log"
    with open(log_path, 'r', encoding='UTF-8') as log:
        for line in log:
            if line.upper().find('ERROR') != -1:
                return line
    return None

def wait_for_keyword_from_log(path, keyword, max_retry_time=300):
    '''
    Wait for a keyword from a log file
    '''
    retry_time = 0
    while retry_time < max_retry_time:
        try:
            with open(path, 'r', encoding='UTF-8') as log_file:
                # check if keyword exists in any line
                for line in log_file:
                    if keyword in line:
                        print(f'Keyword: {keyword} found in file.')
                        return True
                print('Keyword not found. Retrying...')
        except FileNotFoundError:
            print(f"File {path} not found. Retrying...")
        time.sleep(2)
        retry_time += 1
    print('Reached max retry time. Keyword not found.')
    return False

def get_address_from_file(path, title):
    '''
    Get the address from a file.
    '''
    pattern = fr"({title} deployed at )((0x)?[0-9a-fA-F]{{40}})"
    with open(path, 'r', encoding='UTF-8') as log_file:
        lines = log_file.readlines()
        for line in lines:
            result = re.search(pattern, line)
            if result is not None:
                # assuming you want to return the address that follows the title
                return result.group(2)
    return None


def check_log_state(node_idx, log_type, log_name, **kwargs):
    """
    Retrieve information for a given log type from the log
    :param node_idx: Node index to identify the correct log file
    :param log_type: Type of log to search for
    :param kwargs: Unlimited named parameters for matching specific log entry attributes
    :return: True if a matching log entry is found, False otherwise
    """
    log_dic = {
        'DKGGroupingAvailable': ["epoch", "index"],
        'TaskReceived': ["task_type"],
    }

    log_path = f"crates/arpa-node/log/running/node{node_idx}.log"
    with open(log_path, 'r', encoding='UTF-8') as process:
        for line in process:
            try:
                log_json = json.loads(line)
                if log_json["message"]["log_type"] == log_type:
                    # Check if all required keys for this log_type are present and match the kwargs
                    required_keys = log_dic.get(log_type, [])
                    all_match = True
                    for key in required_keys:
                        if key in kwargs:
                            if str(log_json["message"][log_name].get(key)) != str(kwargs[key]):
                                all_match = False
                                break
                        else:
                            # If a required key is not in kwargs, check if it's present in the log; if not, no match
                            if key not in log_json["message"][log_name]:
                                all_match = False
                                break
                    if all_match:
                        return True
            except (ValueError, TypeError):
                continue
        return False
    


def wait_for_state(log_type, log_name, node_process_list, need_all=True, **kwargs):
    """
    Check if all nodes have a log and state. If need_all is True, return True only if all nodes have the state
    after the retry period; otherwise, return False. If need_all is False, return True once one node has the state.
    """
    retry_time = 300
    node_states = [False] * len(node_process_list)  # Track state for each node

    for _ in range(retry_time):
        for node_idx in range(1, len(node_process_list) + 1) :
            if not node_states[node_idx - 1]:  # Check only if the node doesn't have the state yet
                have_state = check_log_state(node_idx, log_type, log_name, **kwargs)
                node_states[node_idx - 1] = have_state  # Update the state

                if not need_all and have_state:  # If need_all is False and one node has the state, return True
                    clear_log()
                    return True

        if need_all and all(node_states):  # If need_all is True and all nodes have the state, return True
            clear_log()
            return True
        time.sleep(1)
    # Print the nodes that don't have the state
    print(f"Nodes that don't have the state: {[i + 1 for i, x in enumerate(node_states) if not x]}")
    return False  # Return False if the conditions are not met after all retries
    
def wait_for_state_one_node(log_type, log_name, node_idx, **kwargs):
    """
    Check if one node has a log and state. Return True if the node has the state after the retry period; otherwise, return False.
    """
    retry_time = 500
    for _ in range(retry_time):
        have_state = check_log_state(node_idx, log_type, log_name, **kwargs)
        if have_state:
            return True
        time.sleep(1)
    return False  # Return False if the conditions are not met after all retries