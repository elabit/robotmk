*** Test Cases ***

TestCase 1 Unwrapped False Assertion
	# Keyword fails, Test fails. 
	# Test needs to have the FAIL message of the keyword. 
	KeywordWithFalseAssertion

TestCase 2 Wrapped False Assertion
	# RF won't show whis msg on the node_top (=test) because the keyword is wrapped. 
	${passed}=  Run Keyword And Return Status  KeywordWithFalseAssertion

TestCase 3 UnWrapped Fail
	FailWithMessage

TestCase 4 Wrapped Fail
	# RF won't show whis msg on the node_top (=test) because the keyword is wrapped. 
	${passed}=  Run Keyword And Return Status  FailWithMessage

TestCase 5 WithTestMessage
	# Sets the tests "text"
	Set Test Message  This is a custom test message.
	
*** Keywords ***

KeywordWithFalseAssertion
	Should Be Equal  1  2  msg=This assertion failed.

FailWithMessage
	Fail  msg=This is the message of a thrown Fail.