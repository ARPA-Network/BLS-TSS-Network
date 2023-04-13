"""
Util functions for the Arpa test environment.
"""
import os
from dotenv import load_dotenv


def make_list(*args):
    """
    Make a list from the input.
    args: accept any number of arguments, which are then gathered into a tuple. 
    """
    return list(args)

def check_group_status(group):
    """
    Check all particialPubKeys are not empty.
    group: group to check.
    """
    memberList = group[4]
    for member in memberList:
        if len(member[1]) == 0:
            return False
    return True

def has_equal_value(value, *args):
    """
    Check if any of the arguments after the first one have the same value as the first argument.
    :param value: The first value to compare against.
    :param args: The rest of the values to check.
    :return: True if any of the values after the first have the same value as the value, False otherwise.
    """
    return any(str(arg) == str(value) for arg in args)

def get_value_from_env(name):
    """
    Get value from .env file.
    """
    load_dotenv("tests/scenarios/.env")
    value = os.environ.get(name)
    return value

def set_value_to_env(name, value):
    """
    Set value to .env file.
    """
    load_dotenv("tests/scenarios/.env")
    os.environ[name] = value
    value = os.environ.get(name)
    print("Set value to env: ", name, value)
    os.system("source contracts/.env")
