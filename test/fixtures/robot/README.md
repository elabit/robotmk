# Contents of this folder

This folder contains several robot test suites. 

## Folder names

The name of each folder contains information about the number of suites/test in it. 

For example, the folder name `1S_3S_2S_3T` means: 

* one root suite, containing
* three subsuites, each containing
* two subsuites, each containing
* three tests

The name `1S_3Snok_2S_3T` implies that on the second suite level an error will be raised.

## How to add new tests

### 1. create Robot suite
First, create a new robot suite in a folder following the naming convention above. This folder can contain any number of suites/tests. 
You should use suite/test/keyword names which are distinguishable, depending on the kind of test you want to write.

### 2. create result data
The second step is to create the result data. The following command executes all robot tests and
converts the XML result into JSON:  

    cd test/fixtures/robot
    make all

This will create a file `output.json` for each tests suite. The JSON exactly represents the data which are feeded into the CheckMK check ("list of lists").

### 3. Define the expected data
Each test suite folder must contain a file `expected.py` which defines what the test should expect.

```
[
    # discovery_suite_level 0
    {
        'suites': ['1S 3S 2S 3T'],
    },
    # discovery_suite_level 1
    {
        'suites': ['Subsuite1', 'Subsuite2', 'Subsuite3'],
    },
    # discovery_suite_level 2
    {
        'suites': ['Sub1 suite1', 'Sub1 suite2', 'Sub2 suite1', 'Sub2 suite2', 'Sub3 suite1', 'Sub3 suite2'],
    },
]
```

### 4. Write the test
#### Inventory tests

In `test_robotmk_check.py` there is a list `inventory_test_params`. Each item in this list will generate _one_ test.
The tuple elements of each item are

* name of the suite folder in `test/fixtures/robot/`
* discovery suite level

```
inventory_test_params = [
    ('1S_3T', 0),
    ('1S_3S_2S_3T', 0),
    ('1S_3S_2S_3T', 1),
]
```

