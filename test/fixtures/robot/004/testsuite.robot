*** Test Cases ***
Test1
	KwNested4

*** Keywords ***
KwNested4
	KwNested3
KwNested3
	KwNested2
KwNested2
	KwNested
KwNested
	# This message should appear on the test !
	Fail  msg=Error during Login!
