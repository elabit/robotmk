*** Test Cases ***

TestCase One
	${passed}=  Run Keyword And Return Status  Keyword With A False Assertion
	Set Test Message  This Test has failed... 

Test Case Two 
	Should Be Equal  2  2
	Set Test Message  Test showed that both numbers are equal. 
*** Keywords ***

Keyword With A False Assertion
	Should Be Equal  1  2  msg=I am sure that those numbers are not equal.