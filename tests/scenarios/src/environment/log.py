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
    return 'Can not find the keyword in log within limit time.'
