*** Variables ***

${ARG1}=  NONE
${ARG2}=  NONE

*** Test Cases ***

Test One
    Log  I am failing right now... 
    Set Test Message  ${ARG1} ${ARG2}
    IF  False
        Run Keyword  Log  This keyword never runs. 
    END