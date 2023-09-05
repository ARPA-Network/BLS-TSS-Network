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
    ${result} =    Run    cargo test --all -- --test-threads=1 --nocapture
    Should Contain    ${result}    result: ok


*** Test Cases ***
Run Normal Process
    Repeat Keyword    1    Unit Test