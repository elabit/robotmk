*** Test Cases ***
Test4 sleep and custom test msg
	Sleepingkw  1
	Set Test Message  I have slept only one second.
	
Test1 wrong assertion default msg
	Compare Numbers with default msg  1  2
Test2 wrong assertion custom msg
	Compare Numbers with custom msg  1  2
Test3 wrong assertion wrapped
	${passed}=  Run Keyword And Return Status  Compare Numbers with custom msg  1  2

Test5 with sleep and custom test msg
	Sleepingkw  2
	Set Test Message  I have slept two seconds.

Sleeptests
	Sleepingkw  1
	Sleepingkw  1
	Sleepingkw  1

*** Keywords ***
Compare Numbers with default msg
	[Arguments]  ${NO1}  ${NO2}
	Should Be Equal As Integers		${NO1}  ${NO2}
Compare Numbers with custom msg
	[Arguments]  ${NO1}  ${NO2}
	Should Be Equal As Integers		${NO1}  ${NO2}  Custom message, no, these numbers are not equal!

Sleepingkw
	[Arguments]  ${SEC} 
	Sleep  ${SEC}
