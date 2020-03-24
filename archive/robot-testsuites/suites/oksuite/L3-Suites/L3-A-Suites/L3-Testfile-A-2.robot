*** Keywords ***
MySleep
	Sleep	0.1

MySleepSleep
	MySleep

MySleepSleepSleep
	MySleepSleep

*** Test Cases ***
Test1 - Sleep
	Sleep 	0.1
Test2 - 1Nested Sleep
	MySleep
Test3 - 2 Nested Sleeps
	MySleepSleep
Test4 - 3 Nested Sleeps
	MySleepSleepSleep
