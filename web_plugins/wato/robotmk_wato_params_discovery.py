#!/usr/bin/python

# (c) 2020 Simon Meggle <simon.meggle@elabit.de>

# This file is part of Robotmk
# https://robotmk.org
# https://github.com/simonmeggle/robotmk

# Robotmk is free software;  you can redistribute it and/or modify it
# under the  terms of the  GNU General Public License  as published by
# the Free Software Foundation in version 3.  This file is distributed
# in the hope that it will be useful, but WITHOUT ANY WARRANTY;  with-
# out even the implied warranty of  MERCHANTABILITY  or  FITNESS FOR A
# PARTICULAR PURPOSE. See the  GNU General Public License for more de-
# ails.  You should have  received  a copy of the  GNU  General Public
# License along with GNU Make; see the file  COPYING.  If  not,  write
# to the Free Software Foundation, Inc., 51 Franklin St,  Fifth Floor,
# Boston, MA 02110-1301 USA.

from cmk.gui.i18n import _
from cmk.gui.valuespec import (DropdownChoice, Dictionary, ListOf, TextAscii,
                               Tuple, TextUnicode)

from cmk.gui.plugins.wato import (
    CheckParameterRulespecWithItem,
    rulespec_registry,
    RulespecGroupCheckParametersDiscovery,
    HostRulespec,
)

# TODO: Add logging True/False
# TODO: warn/crit threholds for total_runtime
# TODO: timeout nicht mehr automatisch von executoin int. berechnen lassen

#       _ _
#      | (_)
#    __| |_ ___  ___ _____   _____ _ __ _   _
#   / _` | / __|/ __/ _ \ \ / / _ \ '__| | | |
#  | (_| | \__ \ (_| (_) \ V /  __/ |  | |_| |
#   \__,_|_|___/\___\___/ \_/ \___|_|   \__, |
#                                        __/ |
#                                       |___/

inventory_dict_robotmk_checkname = (
    "robotmk_service_name",
    TextAscii(
        title=_("Change <b>Robotmk service name</b>"),
        allow_empty=True,
        size=25,
        help=_("""
        A dedicated service is created on each Robotmk client to monitor the staleness 
        of suite statefiles, fatal results, Robotmk version etc.<br>
        Use this setting to override the name of this service. """),
        default_value="Robotmk",
    ))


def _valuespec_inventory_robotmk_rules():
    return Dictionary(
        title=_("Robot Framework Service Discovery"),
        elements=[
            (
                "robot_discovery_settings",
                ListOf(
                    Tuple(elements=[
                        TextAscii(
                            title=("Root suite pattern"),
                            allow_empty=True,
                            size=25,
                            default_value=".*",
                            help=
                            _("Define a regular expression for the root suite in the Robot result you want to set the <b>discovery level</b>. To find out the root suite name, open output.xml of the Robot test and search for the very first suite tag."
                              )),
                        DropdownChoice(
                            title=("Level"),
                            choices=[
                                ("0",
                                 _("0 - create one service from the top result element"
                                   )),
                                ("1",
                                 _("1 - create service(s) from each result element 1 level deeper"
                                   )),
                                ("2",
                                 _("2 - create service(s) from each result element 2 levels deeper"
                                   )),
                                ("3",
                                 _("3 - create service(s) from each result element 3 levels deeper"
                                   )),
                            ],
                            help=
                            _(u"Each Robot result consists of one suite which is either the "
                              u".robot file name or the folder name containg the tests.<br>"
                              u"By default, Robotmk creates 1 service from this single root node.<br>"
                              u"Choosing another level enables you to <b>split the Robot result</b> into as many services as you want.<br>"
                              u"This is perfect for <b>suites</b> and <b>tests</b>. Even if possible, you should <i>not</i> create services from <b>keywords</b>!"
                              ),
                        ),
                        TextAscii(
                            title=("Node Blacklist"),
                            allow_empty=True,
                            size=25,
                            default_value="",
                            help=
                            _("By default, Robotmk will create services for <i>all</i> nodes on the discovery level. A <b>blacklist</b> pattern selectively hinders Robotmk to inventorize certain services.<br>"
                              "Note: An empty string is interpreted as an empty blacklist = inventorize all (default)."
                              )),
                    ]),  # Tuple
                    title=_("Discovery level of services from Robot output"),
                )  # ListOf
            ),
            (
                "robot_service_prefix",
                ListOf(
                    Tuple(elements=[
                        TextAscii(
                            title=("Root suite pattern"),
                            allow_empty=True,
                            size=25,
                            default_value=".*",
                            help=
                            _("Define a regular expression for the root suite in the Robot result you want to set the <b>service name prefix</b>. To find out the root suite name, open output.xml of the Robot test and search for the very first suite tag."
                              )),
                        TextAscii(title=("Service name prefix"),
                                  allow_empty=True,
                                  size=60,
                                  default_value=
                                  "Robot Framework E2E $SUITEID$SPACE-$SPACE",
                                  help=_("""
                                How Robot service names of discovered items should start. The following Variables can be used (usage: <tt>$VAR</tt> or <tt>${VAR}</tt>):<br>
                                <tt>${PATH}</tt>  -  Name of Robot suite directory or <tt>.robot</tt> file<br>
                                <tt>${SUITENAME}</tt>  -  Name of top level suite (usually same name as path)<br>
                                <tt>${TAG}</tt>  -  Suite tag<br>
                                <tt>${SUITEID}</tt>  -  short for <tt>${PATH}_${TAG}</tt><br>
                                <tt>${SPACE}</tt>  -  Use this if there should be a space between the prefix and the item name<br><br>
                                The default format string is "<tt>Robot Framework E2E $SUITEID$SPACE-$SPACE</tt>".
                            """)),
                    ]),  # Tuple_elements
                    title=_("Naming rules for discovered Robot services"),
                )  # ListOf
            ),
            inventory_dict_robotmk_checkname,
        ],  # elements
    )


rulespec_registry.register(
    HostRulespec(
        # lib/python/cmk/gui/watolib/rulespecs.py
        group=RulespecGroupCheckParametersDiscovery,
        match_type="dict",
        name="inventory_robotmk_rules",
        valuespec=_valuespec_inventory_robotmk_rules,
    ))
