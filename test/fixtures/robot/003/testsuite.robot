*** Test Cases ***

TestCase 1 CheckWrappedKeyword
	${passed}=  Run Keyword And Return Status  Keyword With A False Assertion

TestCase 2 CustomTestMessage
	Should Be Equal  2  2
	Set Test Message  This is a custom test message.
	
*** Keywords ***

Keyword With A False Assertion
	Should Be Equal  1  2  msg=This is a custom message for kw exception.