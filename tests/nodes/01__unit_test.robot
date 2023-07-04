*** Settings ***
Library             Process
Library             String
Library             OperatingSystem

*** Variables ***
${CRATES_PATH}                         crates/

*** Keywords ***

Unit Test
    [Documentation]
    ...    Run unit test
    ${result} =    Run Process    cargo    test    --no-fail-fast    --    --test-threads=1    cwd=${CRATES_PATH}    stdout=PIPE    stderr=PIPE
    Log Many    stdout=${result.stdout}
    #Should Not Contain    ${result.stdout}    FAIL


*** Test Cases ***
Run Normal Process
    Repeat Keyword    1    Unit Test