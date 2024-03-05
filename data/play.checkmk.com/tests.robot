*** Settings ***
# Write a descriptive documentation string.
Documentation       This is a test suite for the Checkmk demo site.
...                 It opens the Checkmk demo site and takes screenshots of the dashboards.
...                 The suite is designed to run in a chromium browser, but can also run in other browsers like Firefox.

# Instead of saving the screenshots to the file system, the screenshots are embedded in the log file.
Library             Browser    run_on_failure=Take Screenshot \ EMBED \ fileType=jpeg \ quality=50

# The initialization step is executed before the test cases.
Suite Setup         Suite Initialization


*** Variables ***
${URL}      https://play.checkmk.com
${HOST}     windows-server.company.com


*** Test Cases ***
Open Host Details
    [Documentation]    Open the details of a host and takes a screenshot of the details page.
    All Hosts
    All Services Of Host    ${HOST}

Open Dashboards
    [Documentation]    Open the dashboards and takes a screenshot of them.
    Linux Hosts
    Windows Hosts


*** Keywords ***
Suite Initialization
    Start Browser
    Accept All Cookies
    Dismiss Tour

Accept All Cookies
    ${cookiebanner}=    Get Element States    text=Accept all    then    bool(value & visible)
    IF    ${cookiebanner}    Click    text=Accept all

Start Browser
    New Browser
    ...    browser=chromium
    ...    headless=false
    ...    args=['--start-maximized']
    ...    slowMo=0.5
    New Context    locale=en-US    viewport=${None}
    Add Cookie
    ...    cmk_demo
    ...    kCGgLDI87dmLp48QOLvg
    ...    domain=play.checkmk.com
    ...    path=/play/
    New Page    ${URL}

Dismiss Tour
    ${appcues}=    Get Element States    div.appcues    then    bool(value & visible)
    IF    ${appcues}    Click    div.appcues iframe >>> a[data-step="skip"]

All Hosts
    Click    text="Monitor"
    Wait For Elements State    text="All hosts"
    Click    text="All hosts"
    Take A Screenshot    iframe[name="main"] >>> div#dashboard

Linux Hosts
    Click    text="Monitor"
    Wait For Elements State    text="Linux hosts"
    Click    text="Linux hosts"
    Sleep    3
    Take A Screenshot    iframe[name="main"] >>> div#dashboard

Windows Hosts
    Click    text="Monitor"
    Wait For Elements State    text="Windows hosts"
    Click    text="Windows hosts"
    Sleep    3
    Take A Screenshot    iframe[name="main"] >>> div#dashboard

All Services Of Host
    [Arguments]    ${host}
    ${oldprefix}=    Set Selector Prefix    iframe[name="main"] >>>
    Click    text=${host}
    Get Text    div.titlebar    *=    Services of Host ${host}
    Take A Screenshot    div#main_page_content
    Set Selector Prefix    ${oldprefix}

Take A Screenshot
    [Arguments]    ${selector}
    Browser.Take Screenshot    EMBED    selector=${selector}    fileType=jpeg    quality=50
