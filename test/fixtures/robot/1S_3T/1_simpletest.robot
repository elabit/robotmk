*** Keywords ***
MySleep
	[Arguments]	${time}
	Sleep	${time}

MySleepSleep
	[Arguments]	${time}
	MySleep	${time}

MySleepSleepSleep
	[Arguments]	${time}
	MySleepSleep	${time}

*** Test Cases ***
Test1 - Sleep
	Sleep 	0.1
Test2 - 1Nested Sleep
	MySleep	0.1
Test3 - 2 Nested Sleeps
	MySleepSleep	0.1
Test4 - 3 Nested Sleeps
	MySleepSleepSleep	0.3  