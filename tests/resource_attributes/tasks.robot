*** Settings ***
Documentation       Checks the resource attributes the scheduler passes to robot

Library             OperatingSystem


*** Test Cases ***
Resource Attributes Reach Robot
    ${attributes}=    Get Environment Variable    CHECKMK_RESOURCE_ATTRIBUTES
    Should Be Equal    ${attributes}    ${EXPECTED}
