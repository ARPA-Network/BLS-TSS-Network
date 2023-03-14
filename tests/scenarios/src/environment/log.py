import subprocess
import json
import time
import node

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


def get_keyword_from_log(process, keyword):
    """
    Run a command as a sub-process and check the logs for a given keyword

    :param process: sub-process object
    :param keyword: keyword to look for in the log
    :return: dictionary with the relevant information found for the keyword
    """
    max_retry = 10
    for i in range(max_retry):
        log_info = get_log_info(iter(process.stdout.readline, b''), keyword)

        if log_info is not None:
            return log_info 

        time.sleep(1)

    return None

# pro = node.start_node(1)
# print(get_keyword_from_log(pro, "node_register"))
