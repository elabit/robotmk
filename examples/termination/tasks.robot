*** Settings ***
Library             ${CURDIR}/lib/create_child.py

*** Test Cases ***
Spawn Child
    [Setup]      create_child.setup      ${RESOURCE}
    create_child.spawn  ${FLAG_FILE}
    [Teardown]   create_child.teardown   ${RESOURCE}
