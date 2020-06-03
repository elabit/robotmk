*** Settings ***
Library  OperatingSystem

*** Test Cases ***
Create Directory
   Directory Should Exist  /tmp
   Create Directory  /tmp/robotest
Create File
   Directory Should Exist  /tmp/robotest
   Create File  /tmp/robotest/testfile
Remove File
   Remove File  /tmp/robotest/testfile
Remove Directory
   Remove Directory  /tmp/robotest

*** Keywords ***
