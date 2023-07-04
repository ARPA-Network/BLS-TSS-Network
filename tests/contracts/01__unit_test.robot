*** Settings ***
Library             Process
Library             String
Library             OperatingSystem

*** Variables ***
${CONTRACT_PATH}                         contracts/

*** Keywords ***

Unit Test
    [Documentation]
    ...    Run unit test
    ${result} =    Run Process    forge    test    --gas-price    1000000000    -vvvv    cwd=${CONTRACT_PATH}    stdout=PIPE    stderr=PIPE
    Log Many    stdout=${result.stdout}
    #Should Not Contain    ${result.stdout}    FAIL


*** Test Cases ***
Run Normal Process
    Repeat Keyword    1    Unit Test