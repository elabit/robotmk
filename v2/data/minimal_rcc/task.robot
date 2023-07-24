*** Settings ***
Documentation     Check Python version to ensure rcc is working correctly.


Library  ${EXECDIR}/lib/check.py  WITH NAME  check

*** Test Cases ***

T1: Python Version
        ${result}=  check.version
        ${static}=  check.static
        Should Be Equal As Strings  ${result}  ${static}
