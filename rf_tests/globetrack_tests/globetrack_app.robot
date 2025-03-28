*** Settings ***

Documentation       GlobeTrack is a fake application. This suite just produces
...  a lot of services in Checkmk. 

Library       random

*** Test Cases ***

Successful Login
    CPU Random Sleep  1  3

Validate Expense Submission
    CPU Random Sleep  2  3

Flight Booking Functionality
    CPU Random Sleep  4  8

Password Reset Form
    CPU Random Sleep  3  6

Multi-Currency Expense Reporting
    CPU Random Sleep  2  3

Travel Itinerary Generation
    CPU Random Sleep  6  8

User Role Access Control
    CPU Random Sleep  3  5



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
    Sleep   ${total_sleep}