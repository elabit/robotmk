*** Settings ***
Library     RPA.Excel.Files
Library     RPA.Tables
Library     RPA.Windows
Library     String
Library     Collections
Library     RPA.FileSystem


*** Variables ***
${INPUT_EXCEL}      ${CURDIR}${/}calculations.xlsx
${OUTPUT_EXCEL}     ${OUTPUT_DIR}${/}calculation_results.xlsx


*** Tasks ***
Count My Veggies
    [Documentation]    A sample robot that reads two columns of input and outputs calculations
    Check Veggies Excel And Start Calculator
    ${inputs}=    Read Veggies Excel
    ${outputs}=    Count Veggie Totals    ${inputs}
    Save Veggie Results Excel    ${outputs}
    Close Window    name:Calculator


*** Keywords ***
Read Veggies Excel
    [Documentation]    Reads the Excel sheet for veggies
    Open Workbook    ${INPUT_EXCEL}
    ${inputs}=    Read Worksheet As Table    Sheet1    ${TRUE}    ${TRUE}
    Close Workbook
    RETURN    ${inputs}

Count Veggie Totals
    [Documentation]    Counts the total amounts with Calculator Application
    [Arguments]    ${table}
    ${totals}=    Create List
    FOR    ${row}    IN    @{table}
        Input Number To Calc    ${row}[Carrots]
        Click    Calculator - Plus
        Input Number To Calc    ${row}[Turnips]
        Click    Calculator - Equals
        ${total}=    Get Result From Calc
        Append To List    ${totals}    ${total}
    END
    Set Table Column    ${table}    Totals    ${totals}
    RETURN    @{table}

Input Number To Calc
    [Documentation]    Splits the input number into digits and clicks Calculator buttons
    [Arguments]    ${number}
    Control Window    Calculator
    ${cleared}=    Click If Available    Calculator - CE
    IF    not $cleared    Click If Available    Calculator - C
    ${digits}=    Convert To String    ${number}
    ${digit_list}=    Split String To Characters    ${digits}
    FOR    ${digit}    IN    @{digit_list}
        Click    Calculator - ${digit}
    END

Save Veggie Results Excel
    [Documentation]    Writes the Excel sheet for total amounts of veggies
    [Arguments]    ${outputs}
    Create Workbook    ${CURDIR}${/}calculation_results.xlsx    xlsx
    Create Worksheet    Vegetables    ${outputs}    ${TRUE}    ${TRUE}
    Save Workbook    ${OUTPUT_EXCEL}

Click If Available
    [Documentation]    Clicks Windows locator if available
    [Arguments]    ${locator}
    TRY
        Click    ${locator}
        RETURN    ${TRUE}
    EXCEPT
        RETURN    ${FALSE}
    END

Get Result From Calc
    [Documentation]    Reads Calculator's calculation result
    ${result}=    Get Attribute    Calculator - IdCalculatorResults    Name
    ${total}=    Remove String    ${result}    Display is${SPACE}
    ${total}=    Convert To Integer    ${total}
    RETURN    ${total}

Check Veggies Excel And Start Calculator
    ${exists}=    Does File Exist    ${INPUT_EXCEL}
    IF    not $exists    Fail    Missing input: ${INPUT_EXCEL}
    Windows Search    Calculator
