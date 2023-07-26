*** Settings ***
Documentation     Test file for configuring RobotFramework


Library  ${EXECDIR}/lib/add.py  WITH NAME  math

Suite Setup     math.setup
Suite TearDown     math.teardown


*** Variables ***
${expected_result_2}    4


*** Test Cases ***

Addition 1
        ${result}=  math.add  ${20}  ${15}
        Should Be Equal As Integers  ${result}  ${35}

Addition 2
        ${result}=  math.add  ${1}  ${2}
        Should Be Equal As Integers  ${result}  ${expected_result_2}
