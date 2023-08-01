*** Settings ***
Documentation     Test file for configuring RobotFramework


Library  ${CURDIR}/lib/add.py  WITH NAME  math

Suite Setup     math.setup
Suite TearDown     math.teardown


*** Variables ***



*** Test Cases ***

Addition
        ${result}=  math.add  ${20}  ${15}
        Should Be Equal As Integers  ${result}  ${35}
