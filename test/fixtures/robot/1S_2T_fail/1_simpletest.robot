*** Keywords ***
Ten is the same as hundred
	Should Be Equal As Integers		10  100   Numbers are not equal

Hundred is the same as Ten
	Should Be Equal As Integers		100  10   Numbers are not equal

*** Test Cases ***
This first test fails
	Ten is the same as hundred
This second test fails
	Hundred is the same as Ten
