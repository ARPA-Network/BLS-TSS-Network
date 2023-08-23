"""
This module contains functions to retrieve information from the logs
"""
import json
import os
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
            if message.find(keyword) != -1:
                return message
        except ValueError:
            continue
    return None


def get_keyword_from_log(node_idx, keyword, retry_time=30):
    """
    Run a command as a sub-process and check the logs for a given keyword
    :param process: sub-process object
    :param keyword: keyword to look for in the log
    :return: the relevant information found for the keyword
    """
    i =  0
    while i < retry_time:
        i = i + 1
        log_path = f"crates/arpa-node/log/running/node{node_idx}.log"
        with open(log_path, 'r', encoding='UTF-8') as process:
            log_info = get_log_info(process, keyword)
            if log_info is not None:
                return log_info
        time.sleep(1)
    return None


def have_node_got_keyword(keyword, node_process_list, retry_time=10):
    """
    Get a keyword from all nodes
    :param keyword: keyword to look for in the log
    :return: whether the relevant information was found for the keyword
    """
    while retry_time > 0:
        retry_time = retry_time - 1
        node_idx = 1
        while node_idx <= len(node_process_list):
            log_info = get_keyword_from_log(node_idx, keyword, 30)
            if log_info is not None:
                return True
            node_idx = node_idx + 1
    return False


def all_nodes_have_keyword(keyword, node_process_list, retry_time=300):
    """
    Check if all nodes have a keyword in their logs
    :param keyword: keyword to look for in the log
    :return: dictionary with the relevant information found for the keyword
    """
    node_idx = 1
    while node_idx <= len(node_process_list):
        log_info = get_keyword_from_log(node_idx, keyword, retry_time)
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
