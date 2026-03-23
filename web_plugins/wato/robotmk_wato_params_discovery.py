#!/usr/bin/python

# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)
#x

rule_title = "Robotmk v1 Service Discovery"
rule_name = "inventory_robotmk_rule"

# Compatibility layer for Checkmk 2.5+ (v1 API) and older versions (legacy API)
try:
    # Checkmk 2.5+ uses v1 API
    from cmk.rulesets.v1 import Title, Help
    from cmk.rulesets.v1.form_specs import (
        DefaultValue,
        DictElement,
        Dictionary,
        List,
        String,
        SingleChoice,
        SingleChoiceElement,
    )
    from cmk.rulesets.v1.rule_specs import DiscoveryParameters, Topic
    USES_V1_API = True
except ImportError:
    USES_V1_API = False

# Only import legacy API if v1 is not available AND we're in a GUI context
if not USES_V1_API:
    try:
        from cmk.gui.i18n import _
        from cmk.gui.valuespec import (
            DropdownChoice,
            Dictionary,
            ListOf,
            TextAscii,
            Tuple,
            TextUnicode,
            Transform,
        )
        from cmk.gui.plugins.wato import (
            CheckParameterRulespecWithItem,
            rulespec_registry,
            RulespecGroupCheckParametersDiscovery,
            HostRulespec,
        )
        LEGACY_API_AVAILABLE = True
    except ImportError:
        # Running standalone without GUI context
        LEGACY_API_AVAILABLE = False

#       _ _
#      | (_)
#    __| |_ ___  ___ _____   _____ _ __ _   _
#   / _` | / __|/ __/ _ \ \ / / _ \ '__| | | |
#  | (_| | \__ \ (_| (_) \ V /  __/ |  | |_| |
#   \__,_|_|___/\___\___/ \_/ \___|_|   \__, |
#                                        __/ |
#                                       |___/

# =============================================================================
# V1 API Implementation (Checkmk 2.5+)
# =============================================================================
if USES_V1_API:
    def _parameter_form_discovery_robotmk():
        return Dictionary(
            elements={
                "robot_discovery_settings": DictElement(
                    required=False,
                    parameter_form=List(
                        element_template=Dictionary(
                            elements={
                                "pattern": DictElement(
                                    required=True,
                                    parameter_form=String(
                                        title=Title("Root suite pattern"),
                                        prefill=DefaultValue(".*"),
                                        help_text=Help(
                                            "Define a regular expression for the root suite in the Robot "
                                            "result you want to set the discovery level. To find out the root "
                                            "suite name, open output.xml of the Robot test and search for the "
                                            "very first suite tag."
                                        ),
                                    ),
                                ),
                                "level": DictElement(
                                    required=True,
                                    parameter_form=SingleChoice(
                                        title=Title("Level"),
                                        prefill=DefaultValue("level_0"),
                                        elements=[
                                            SingleChoiceElement(
                                                name="level_0",
                                                title=Title(
                                                    "0 - create one service from the top result element"
                                                ),
                                            ),
                                            SingleChoiceElement(
                                                name="level_1",
                                                title=Title(
                                                    "1 - create service(s) from each result element 1 level deeper"
                                                ),
                                            ),
                                            SingleChoiceElement(
                                                name="level_2",
                                                title=Title(
                                                    "2 - create service(s) from each result element 2 levels deeper"
                                                ),
                                            ),
                                            SingleChoiceElement(
                                                name="level_3",
                                                title=Title(
                                                    "3 - create service(s) from each result element 3 levels deeper"
                                                ),
                                            ),
                                        ],
                                        help_text=Help(
                                            "Each Robot result consists of one suite which is either the "
                                            ".robot file name or the folder name containing the tests. "
                                            "By default, Robotmk creates 1 service from this single root node. "
                                            "Choosing another level enables you to split the Robot result into "
                                            "as many services as you want. This is perfect for suites and tests. "
                                            "Even if possible, you should not create services from keywords!"
                                        ),
                                    ),
                                ),
                                "blacklist": DictElement(
                                    required=True,
                                    parameter_form=String(
                                        title=Title("Node Blacklist"),
                                        prefill=DefaultValue(""),
                                        help_text=Help(
                                            "By default, Robotmk will create services for all nodes on the "
                                            "discovery level. A blacklist pattern selectively hinders Robotmk "
                                            "to inventorize certain services. Note: An empty string is "
                                            "interpreted as an empty blacklist = inventorize all (default)."
                                        ),
                                    ),
                                ),
                            },
                        ),
                        title=Title("Discovery level of services from Robot output"),
                    ),
                ),
                "robot_service_prefix": DictElement(
                    required=False,
                    parameter_form=List(
                        element_template=Dictionary(
                            elements={
                                "pattern": DictElement(
                                    required=True,
                                    parameter_form=String(
                                        title=Title("Root suite pattern"),
                                        prefill=DefaultValue(".*"),
                                        help_text=Help(
                                            "Define a regular expression for the root suite in the Robot "
                                            "result you want to set the service name prefix. To find out the "
                                            "root suite name, open output.xml of the Robot test and search for "
                                            "the very first suite tag."
                                        ),
                                    ),
                                ),
                                "prefix": DictElement(
                                    required=True,
                                    parameter_form=String(
                                        title=Title("Service name prefix"),
                                        prefill=DefaultValue("Robot Framework E2E $SUITEID$SPACE-$SPACE"),
                                        help_text=Help(
                                            "How Robot service names of discovered items should start. The following "
                                            "Variables can be used (usage: $VAR or ${VAR}):\n"
                                            "${PATH} - Name of Robot suite directory or .robot file\n"
                                            "${SUITENAME} - Name of top level suite (usually same name as path)\n"
                                            "${TAG} - Suite tag\n"
                                            "${SUITEID} - short for ${PATH}_${TAG}\n"
                                            "${SPACE} - Use this if there should be a space between the prefix and the item name\n"
                                            'The default format string is "Robot Framework E2E $SUITEID$SPACE-$SPACE".'
                                        ),
                                    ),
                                ),
                            },
                        ),
                        title=Title("Naming of discovered services"),
                    ),
                ),
                "robotmk_service_name": DictElement(
                    required=False,
                    parameter_form=String(
                        title=Title("Change Robotmk service name"),
                        prefill=DefaultValue("Robotmk"),
                        help_text=Help(
                            "A dedicated service is created on each Robotmk client to monitor the "
                            "staleness of suite statefiles, fatal results, Robotmk version etc. "
                            "Use this setting to override the name of this service."
                        ),
                    ),
                ),
                "htmllog": DictElement(
                    required=False,
                    parameter_form=Dictionary(
                        elements={
                            "last_log": DictElement(
                                required=True,
                                parameter_form=String(
                                    title=Title("Services to create a last log file link for:"),
                                    prefill=DefaultValue(".*"),
                                ),
                            ),
                            "last_error_log": DictElement(
                                required=True,
                                parameter_form=String(
                                    title=Title("Services to create a last error log file link for:"),
                                    prefill=DefaultValue(".*"),
                                ),
                            ),
                        },
                        title=Title("HTML log file integration"),
                        help_text=Help(
                            "Robotmk can display two action icons right of each discovered service which "
                            "allows to open the last error log and the current log. The host must exist "
                            "with the real hostname/FQDN in Checkmk. Robotmk will save max. 2 HTML files "
                            "per discovered suite to save hard disk space. However, it is advised to monitor "
                            "the space usage on $OMD_ROOT/var/robotmk. The regular expressions below here "
                            "define where the service label will be shown. Default = .* = show HTML logs on "
                            "all Robotmk services."
                        ),
                    ),
                ),
            },
        )

    rule_spec_inventory_robotmk = DiscoveryParameters(
    	name=rule_name,
    	topic=Topic.GENERAL,
        title=Title(rule_title),
        parameter_form=_parameter_form_discovery_robotmk,
    )

# =============================================================================
# Legacy API Implementation (Checkmk 2.4 and below)
# =============================================================================
elif LEGACY_API_AVAILABLE:
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
        """Transform data from rules.mk format to WATO GUI format (dict → tuple).
        
        Handles both CMK 2.4 and earlier (dict format) and CMK 2.5+ (may already be tuple).
        """
        # CMK 2.5+ may already provide data as tuple
        if isinstance(data, tuple) and len(data) == 2:
            return data
        
        # CMK 2.4 and earlier: dict format
        if isinstance(data, dict):
            return (
                data.get('last_log', '.*'),
                data.get('last_error_log', '.*'),
            )
        
        # Fallback for any unexpected format
        return ('.*', '.*')

    def back(data):
        """Transform data from WATO GUI format to rules.mk format (tuple → dict).
        
        Converts tuple format to dict for storage in rules.mk.
        """
        # Handle both tuple and list formats
        if isinstance(data, (tuple, list)) and len(data) >= 2:
            return {
                'last_log': data[0],
                'last_error_log': data[1],
            }
        
        # Fallback for unexpected format
        return {
            'last_log': '.*',
            'last_error_log': '.*',
        }


    def _valuespec_inventory_robotmk_rules():
        return Dictionary(
            title=_(rule_title),
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
            name=rule_name,
            valuespec=_valuespec_inventory_robotmk_rules,
        ))

# Test imports when run standalone
if __name__ == "__main__":
    print(f"USES_V1_API: {USES_V1_API}")
    if USES_V1_API:
        print("✓ Successfully imported v1 API")
        print(f"✓ Rule spec created: {rule_spec_inventory_robotmk.name}")
    elif LEGACY_API_AVAILABLE:
        print("✓ Successfully imported legacy API")
        print("✓ Rule spec will be registered in GUI context")
    else:
        print("✗ No API available (likely standalone execution without GUI)")
    print("File loads successfully!")
