*** Settings ***
Documentation       Test file for configuring RobotFramework

Library             ${CURDIR}/lib/add.py    WITH NAME    math

Suite Setup         math.setup
Suite Teardown      math.teardown


*** Test Cases ***
Addition One
    ${result}=    math.add    ${20}    ${5}
    Should Be Equal As Integers    ${result}    ${25}

Addition Two
    ${result}=    math.add    ${20}    ${15}
    Should Be Equal As Integers    ${result}    ${35}

Addition Three
    ${result}=    math.add    ${20}    ${25}
    Should Be Equal As Integers    ${result}    ${45}
