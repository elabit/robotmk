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

from cmk.gui.log import logger   

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

inventory_dict_robotmk_htmllog = (
    Tuple(
        title=_("<b>HTML log file</b> integration"),
        help=_("""Robotmk can display two action icons right of each discovered service which allows to open the <b>last error log</b> and the <b>current log</b>.<br>
        The host must exist with the real hostname/FQDN in Checkmk. Robotmk will save max. 2 HTML files per discovered suite to save hard disk space. However, it is advised to monitor the space usage on <tt>$OMD_ROOT/var/robotmk</tt>.<br>
        The regular expressions below here define where the service label will be shown. <br>Default = <tt>.*</tt> = show HTML logs on all Robotmk services."""),
        show_titles=True,
        # orientation="horizontal",
        elements=[
            TextAscii(
                title=("Services to create a last <b>log file</b> link for:"),
                allow_empty=False,
                size=40,
                default_value='.*'
            ),
            TextAscii(
                title=("Services to create a last <b>error log file</b> link for:"),
                allow_empty=False,
                size=40,
                default_value='.*'
            ),
        ],
    )
)

def forth(data):
    logger.critical("forth here ----------------------------")
    logger.critical(data)
    if type(data) is dict and len(data) == 2:
        return (
            data.get('last_log', '.*'),
            data.get('last_error_log', '.*'),
        )
    else: 
        return ('.*', '.*')

    return ('foo', 'bar')

def back(data):
    logger.critical("back here ----------------------------")
    logger.critical(data)
    return {
        'last_log': data[0],
        'last_error_log': data[1],
    }
    # return ('back', 'baz')


def _valuespec_inventory_robotmk_rules():
    return Dictionary(
        title=_("Robot Framework Service Discovery"),
        # optional_keys=['robot_discovery_settings','robot_service_prefix','robotmk_service_name'],
        elements=[
            (
                "robot_discovery_settings",
                ListOf(
                    Tuple(elements=[
                        TextAscii(
                            title=("Root suite pattern"),
                            allow_empty=True,
                            size=40,
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
                            size=40,
                            default_value="",
                            help=
                            _("By default, Robotmk will create services for <i>all</i> nodes on the discovery level. A <b>blacklist</b> pattern selectively hinders Robotmk to inventorize certain services.<br>"
                              "Note: An empty string is interpreted as an empty blacklist = inventorize all (default)."
                              )),
                    ]),  # Tuple
                    title=_("<b>Discovery level</b> of services from Robot output"),
                )  # ListOf
            ),
            (
                "robot_service_prefix",
                ListOf(
                    Tuple(elements=[
                        TextAscii(
                            title=("Root suite pattern"),
                            allow_empty=True,
                            size=40,
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
                    title=_("<b>Naming</b> of discovered services"),
                )  # ListOf
            ),
            inventory_dict_robotmk_checkname,
            ('htmllog', 
            Transform(
                inventory_dict_robotmk_htmllog,
                # read from rules.mk, present in WATO
                forth = forth,
                # read from WATO
                back = back,
            ))
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
