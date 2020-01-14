#!/usr/bin/python
from cmk.gui.i18n import _
from cmk.gui.valuespec import (
    Checkbox,
    DropdownChoice,
    Dictionary,
    ListChoice,
    ListOf,
    TextAscii,
    TextUnicode,
    Tuple,
)

from cmk.gui.plugins.wato import (
    CheckParameterRulespecWithItem,
    rulespec_registry,
    RulespecGroupCheckParametersDiscovery,
    RulespecGroupCheckParametersStorage,
    HostRulespec,
)

def _valuespec_inventory_robot_rules():
    return Dictionary(
        title=_("Robot Framework Test Discovery"),
        elements=[
            ( "discovery_suite_level",
                DropdownChoice(
                    title = _("Discovery suite level"),
                    help=_(
                        u"Each Robot result consists of one suite which is the folder name of the test(s)."
                        u"Below that, you have sub-suites for each robot test file and/or for each subfolder."
                        u"Choosing level 0 will create 1 service which will reflect all tests and suites within this folder."
                        u"A level of 1 or higher will create 1 or more services, depending on the number of test files/folders"
                        u"within that directory level."
                        ),
                    choices = [
                        ( "0"  , _("Directory level 0 (1 service with root folder name)") ),
                        ( "1"  , _("Directory level 1 (n services for each test file/subfolder") ),
                        ( "2"  , _("Directory level 2 (\"-\")") ),
                        ( "3"  , _("Directory level 3 (\"-\")") ),
                    ]
            )),
        ],
    )


rulespec_registry.register(
    HostRulespec(
        group=RulespecGroupCheckParametersDiscovery,
        match_type="dict",
        name="inventory_robot_rules",
        valuespec=_valuespec_inventory_robot_rules,
    ))


#        elements = [
#            ( "discovery_suite_level",
#                DropdownChoice(
#                    title = _("discovery_suite_level"),
#                    choices = [
#                        ( "0"  , _("0 - Dirname of the test -> one serviceX") ),
#                        ( "1"  , _("1 - Names of all dirs and tests in the root folder -> many services") ),
#                        ( "2"  , _("2 - Names of all dirs and tests in the layer 1 subfolder -> many services") ),
#                    ]
#            ),
#        ]
#
#    ),
