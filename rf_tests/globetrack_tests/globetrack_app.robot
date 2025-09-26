*** Settings ***

Documentation       GlobeTrack is a fake application. This suite just produces
...  a lot of services in Checkmk. 

Library       random

*** Test Cases ***

Successful Login
    CPU Random Sleep  1  2

Validate Expense Submission
    CPU Random Sleep  1  2

Flight Booking Functionality
    CPU Random Sleep  1  2

Password Reset Form
    CPU Random Sleep  1  2

Multi-Currency Expense Reporting
    CPU Random Sleep  1  2

Travel Itinerary Generation
    CPU Random Sleep  1  2

User Role Access Control
    CPU Random Sleep  1  2



*** Keywords ***

Random Sleep
    [Arguments]   ${sec1}  ${sec2}
    ${randomWait}=    Evaluate    random.uniform(${sec1},${sec2})    random
    Sleep    ${randomWait}

CPU Random Sleep 
    [Arguments]   ${sec1}  ${sec2}
    ${cpu_load}    Evaluate    psutil.cpu_percent(interval=1)    psutil
    ${base_sleep}    Evaluate    ${cpu_load} / 10
    ${random_addition}    Evaluate    random.uniform(${sec1},${sec2})    random
    ${total_sleep}    Evaluate    ${base_sleep} + ${random_addition}
    #Sleep   ${base_sleep}
    Sleep   ${total_sleep}
