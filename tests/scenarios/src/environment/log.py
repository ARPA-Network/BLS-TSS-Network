"""
This module contains functions to retrieve information from the logs
"""
import json
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
            pass

    return None


def get_keyword_from_log(process, keyword, retry_time=30):
    """
    Run a command as a sub-process and check the logs for a given keyword
    :param process: sub-process object
    :param keyword: keyword to look for in the log
    :return: dictionary with the relevant information found for the keyword
    """
    max_retry = retry_time
    i =  0
    while i < max_retry:
        i = i + 1
        log_info = get_log_info(iter(process.stdout.readline, b''), keyword)
        if log_info is not None:
            return log_info
        time.sleep(1)
    return None

def have_node_got_keyword(keyword, node_process_list, retry_time=30):
    """
    Get a keyword from all nodes
    :param keyword: keyword to look for in the log
    :return: dictionary with the relevant information found for the keyword
    """
    print(len(node_process_list))
    for node in node_process_list:
        log_info = get_keyword_from_log(node, keyword, retry_time)
        if log_info is not None:
            return True
    return False

def all_nodes_have_keyword(keyword, node_process_list, retry_time=30):
    """
    Check if all nodes have a keyword in their logs
    :param keyword: keyword to look for in the log
    :return: dictionary with the relevant information found for the keyword
    """
    for node in node_process_list:
        log_info = get_keyword_from_log(node, keyword, retry_time)
        if log_info is None:
            return False
    return True

def clear_log():
    """
    Clear the node log file
    """
    open('crates/arpa-node/log/running/node.log', 'w', encoding='UTF-8').close()
