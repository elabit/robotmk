*** Settings ***
Library             ${CURDIR}/lib/create_child.py

*** Test Cases ***
Spawn Child
    [Setup]      create_child.setup      ${RESOURCE}
    create_child.Spawn
    [Teardown]   create_child.teardown   ${RESOURCE}
