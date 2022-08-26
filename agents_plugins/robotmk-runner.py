#!/usr/bin/env python3
# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

try:
    # Import the main Robotmk functions from the same directory (Windows)
    from robotmk import *
except ImportError:
    # If the import fails, try to import robotmk form the parent directory (Linux)
    # This is the case when the runner gets scheduled asynchronously on Linux where
    # it is saved in a subfolder
    import sys, os
    sys.path.insert(1, os.path.join(sys.path[0], '..'))
    from robotmk import *


def main():
    test_for_modules()
    RMKPlugin.get_args()
    rmk = RMKrunner()
    cmdline_suites='all' # TBD: start suites from cmdline
    rmk.run_suites(cmdline_suites)
    rmk.loginfo("... Quitting Runner, bye. ---") 
    # It is important to write at least one byte to the agent so that it can save this
    # as a state with a cache_time. 
    print('')    

if __name__ == '__main__':
    main()
else:
    # when imported as module
    import mergedeep
    import robot
    import yaml
    from dateutil import parser
