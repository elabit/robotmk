Install checkMK pytest module: 

```
pip install -e /python-pytest-check_mk/
```

## Test data structure

`output_depth_ suites/keywords`

`runtime_thresolds_ suites/tests/keywords`



```
Lv  LvRe
0   -       Mkdemo/
1   0           A-Suites/ ****
2   1               A-suite1/
3   999                 test-A-1-1
4   0                       keyword
5   1                           keyword
6   2                               keyword
                                keyword
                                keyword
                            kw
                            kw
                        test-A-1-2
                        test-A-1-3
                        test-A-1-4
                    A-suite2/
                        test-A-2-1
                        test-A-2-2
                        test-A-2-3
                        test-A-2-4
                    A-suite3/
                    ...
                    ...
                B-Suites/
                    B-suite1/
                        test-B-1-1
                        test-B-1-2
                        test-B-1-3
                        test-B-1-4
                    B-suite2
                    ...
                    ...
                C-Suites/
                    C-suite1/
                    ...
                    ...
```