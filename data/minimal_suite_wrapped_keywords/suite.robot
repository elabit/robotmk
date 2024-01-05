*** Test Cases ***
Test With An Ignored Error
    Run Keyword And Ignore Error    Fail
    Log    The test does not fail because the failed keyword was ignored.

Test Which continues After Error
    Run Keyword And Continue On Failure    Fail
    Log    The test fails, but continued after a failure

Test Which Gets Keyword Status
    ${passed}=    Run Keyword And Return Status    Should Be Equal As Integers    1    2
    Log    The test does not fail because only the keyword status was reuested.
