"""
Util functions for the Arpa test environment.
"""
import glob
import os
import signal
from datetime import datetime, timezone
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
    member_list = group[4]
    for member in member_list:
        if len(member[1]) == 0:
            return False
    return True

def has_equal_value(value, *args):
    """
    Check if any of the arguments after the first one have the same value as the first argument.
    :param value: The first value to compare against.
    :param args: The rest of the values to check.
    :return: True if any of the values after the first have the same value as the value,
        False otherwise.
    """
    return any(str(arg).upper() == str(value).upper() for arg in args)

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
    os.system("cp tests/scenarios/.env contracts/.env")

def list_should_contain(address_list, value):
    """
    Check if the list contains the value.
    """
    print("address_list: ", address_list)
    return str(value).upper() in [str(item).upper() for item in address_list]

def get_amount_count_from_reward_events(event_list, address):
    """
    Get amount from reward events list.
    """
    amount = 0
    for node in event_list:
        node_address = node["args"]["nodeAddress"]
        if str(node_address).upper() == str(address).upper():
            amount += int(node["args"]["arpaAmount"])
    return amount

def get_account_index_from_list(address, account_list):
    """
    Get account index from list.
    """
    for index, account in enumerate(account_list):
        if str(account.address).upper() == str(address).upper():
            return index + 1
    return -1

def kill_process_by_name(name):
    """
    Kill a process by its name.
    """
    if os.name == 'nt': # For Windows
        os.system(f"taskkill /f /im {name}.exe")
    else: # For Linux
        result = os.popen(f"pgrep {name}").read().splitlines()
        for pid in result:
            if pid != "":
                os.kill(int(pid), signal.SIGTERM)
    print(f"{name} process was killed successfully!")

def approximately_equal(val_x, val_y, tolerance = 1000000000):
    """
    Check if two numbers are approximately equal.
    """
    return abs(val_x - val_y) <= tolerance

def convert_timestamp(time_str):
    """
    Strip time string.
    """
    rdt_obj = datetime.strptime(time_str, "%a %b %d %H:%M:%S %Y")

    # Set the time zone to GMT 00:00
    gmt_timezone = timezone.utc
    rdt_obj = rdt_obj.replace(tzinfo=gmt_timezone)

    unix_timestamp = rdt_obj.timestamp()
    return int(unix_timestamp)

def clear_database():
    """
    Clear sqlite database.
    """
    pattern = os.path.join('crates/arpa-node/', '*.sqlite')
    # find all the matching files
    sqlite_files = glob.glob(pattern)
    # iterate over the list of files and remove them
    for file_path in sqlite_files:
        print(f'Deleting file {file_path}')
        os.remove(file_path)
