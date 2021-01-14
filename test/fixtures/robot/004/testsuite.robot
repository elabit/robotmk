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
	Fail  msg=Foo
