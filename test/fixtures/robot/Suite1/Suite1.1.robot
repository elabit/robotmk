*** Settings ***
Library  Process


*** Test Cases ***

Sleep the first time for 5 sec

    sleep  0.1 

Sleep the second time for 120 sec

    sleep  1 

Sleep the third time for random sec
    ${sleptime} =  Run Process  /usr/bin/shuf  -i 10-60  -n 1 
    sleep  ${sleptime.stdout} 
