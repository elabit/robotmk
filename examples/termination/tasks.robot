*** Settings ***
Library             ${CURDIR}/lib/create_child.py

*** Test Cases ***
Spawn Child
    create_child.spawn  ${FLAG_FILE}
