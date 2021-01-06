#!/usr/bin/python

# (c) 2020 Simon Meggle <simon.meggle@elabit.de>

# This file is part of RobotMK
# https://robotmk.org
# https://github.com/simonmeggle/robotmk

# RobotMK is free software;  you can redistribute it and/or modify it
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
from cmk.gui.valuespec import (
    DropdownChoice,
    Dictionary,
    ListOf,
    TextAscii,
    Tuple,
)

from cmk.gui.plugins.wato import (
    CheckParameterRulespecWithItem,
    rulespec_registry,
    RulespecGroupCheckParametersDiscovery,
    RulespecGroupCheckParametersApplications,
    HostRulespec,
)


from cmk.gui.cee.plugins.wato.agent_bakery import (
    RulespecGroupMonitoringAgentsAgentPlugins
)

helptext_listof_testsuites_spooldir="""
    By default (if you do not add any suite), the RobotMK plugin will execute <i>all</i> <tt>.robot</tt> files in the <i>Robot suites directory</i> without any parametrization.<br>
    To specify suites, additional parameters and execution order, click <i>Add test suite</i>.<br>    
    Keep in mind to set a proper <i>cache time</i>. It should be higher than the estimated maximum runtime of all suites."""

helptext_listof_testsuites_cmkasync="""
    By default (if you do not add any suite), the RobotMK plugin will execute <i>all</i> <tt>.robot</tt> files in the <i>Robot suites directory</i> without any parametrization.<br>
    To specify suites, additional parameters and execution order, click <i>Add test suite</i>.<br>    
    Keep in mind to set a proper <i>execution interval</i>. It should be higher than the estimated maximum runtime of all suites."""

helptext_execution_mode_cmk_async="""
    The plugin will be placed within the agent's <tt>plugin</tt> directory.<br>
    It will be <b>executed</b> asynchronously <b>by the agent</b> in the given <i>execution interval</i>.<br>
    The Checkmk agent will read the result from <tt>STDOUT</tt> of the RobotMK plugin.<br><br>
    <b>Use cases</b> for this mode: in general, all Robot tests which can run headless and do not require a certain OS user."""
helptext_execution_mode_spooldir="""
    In this mode there is no plugin execution by the agent; <b>schedule it manually</b> with Jenkins, cron, Windows task scheduler etc.<br>
    The <i>agent plugin cache time</i> should be higher than the scheduling interval.<br>
    The result of the plugin will be written into the <tt>SPOOLDIR</tt> of the Checkmk agent.<br>
    <b>Important note</b>: You need to utilize the rule <i>Deploy custom files to agent</i> to deliver the file package '<i>robotmk-plugin</i>'. The RobotMK plugin will be installed then in the <tt>bin</tt> folder (instead of <tt>plugin</tt>) on the agent.<br><br>
    <b>Use cases</b> for this mode: <br>
      - Applications which need a desktop<br>
      - Applications which require to be run with a certain user account<br>
      - The need for more control about when to execute a Robot test and when not"""

agent_config_cache_time_cmk_async=(
    "cache_time",
    Age(
        title=_("Execution interval (default: 15min)"),
        help=_("Sets the interval in which the Checkmk agent will execute the RobotMK plugin (instead of normal check interval).<br>"
        "The default is 15min but strongly depends on the maximum probable runtime of all <i>test suites</i>. Choose an interval which is a good comprimise between frequency and execution runtime headroom.<br>"
        "To avoid concurrency problems, the Checkmk agent is configured with a fixed plugin timeout of <tt>cache_time - 60s</tt> ."),
        minvalue=1,
        maxvalue=65535,
        default_value=900,
    )
)
agent_config_cache_time_spooldir=(
    "cache_time",
    Age(
        title=_("Cache time (default: 15min)"),
        help=_("The <i>cache time</i> should always be slightly higher than the scheduling interval of the RobotMK plugin in cron, Windows Task Planner, Jenkins, etc.<br>"
        "The scheduling interval in turn should be quite higher than the maximum probable runtime of all <i>test suites</i> to avoid concurency problems. Choose an interval which is a good comprimise between frequency and execution runtime headroom.<br>"),
        minvalue=1,
        maxvalue=65535,
        default_value=900,
    )
)

agent_config_robotdir = (
    "robotdir",
    TextUnicode(
        title=_("Robot suites directory"),
        help=_("By default the RobotMK plugin will search for Robot suites in the following paths:<br>"
                " - <tt>/usr/lib/check_mk_agent/robot</tt> (Linux)<br>"
                " - <tt>C:\\ProgramData\\checkmk\\agent\\robot</tt> (Windows) <br>"
                "You can override this setting if Robot tests are stored in another path. <br>"
                "Windows paths can be given with single backslashes; OS dependent path validation is made during the agent baking.<br>"),
        allow_empty=True,
        size=100,
        default_value=""
    )
) 

# Make the help text of SuitList dependent on the type of execution
def agent_config_listof_testsuites(helptext):
    return (
    "suites",
    ListOf(
        Tuple(elements=[
            TextUnicode(
                title=_("Robot test file/dir name"),
                help=_("Robot Framework can execute <tt>.robot</tt> files as well as nested directories "
                    "which itself contain <tt>.robot</tt> files. All names are expected to be relative to the <i>robot dir</i>."),
                allow_empty=False,
                size=50,
            ), 

            Dictionary(
                elements=[    
                    ("name",
                    TextUnicode(
                        title=_("Top level suite name (<tt>--name</tt>)"),
                        help=_("Set the name of the top level suite. By default the name is created based on the executed file or directory.<br>"
                            "This sets the name of a fresh discovered Robot service; an already existing service will hide away and will be found by the discovery under a new name."),
                        allow_empty=False,
                        size=50,
                    )), 
                    ("suite",
                    ListOfStrings(
                        title=_("Select suites (<tt>--suite</tt>)"),
                        help=_("Select suites by name. <br>When this option is used with"
                                " <tt>--test</tt>, <tt>--include</tt> or <tt>--exclude</tt>, only tests in"
                                " matching suites and also matching other filtering"
                                " criteria are selected. <br>"
                                " Name can be a simple pattern similarly as with <tt>--test</tt> and it can contain parent"
                                " name separated with a dot. <br>"
                                " For example, <tt>X.Y</tt> selects suite <tt>Y</tt> only if its parent is <tt>X</tt>.<br>"),
                        size=40,
                    )),                                                 
                    ("test",
                    ListOfStrings(
                        title=_("Select test (<tt>--test</tt>)"),
                        help=_("Select tests by name or by long name containing also"
                                " parent suite name like <tt>Parent.Test</tt>. <br>Name is case"
                                " and space insensitive and it can also be a simple"
                                " pattern where <tt>*</tt> matches anything, <tt>?</tt> matches any"
                                " single character, and <tt>[chars]</tt> matches one character"
                                " in brackets.<br>"),
                        size=40,
                    )),                                            
                    ("include",
                    ListOfStrings(
                        title=_("Include tests by tag (<tt>--include</tt>)"),
                        help=_("Select tests by tag. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#tagging-test-cases\">About tagging test cases</a>)<br>Similarly as name with <tt>--test</tt>,"
                                "tag is case and space insensitive and it is possible"
                                "to use patterns with <tt>*</tt>, <tt>?</tt> and <tt>[]</tt> as wildcards.<br>"
                                "Tags and patterns can also be combined together with"
                                "<tt>AND</tt>, <tt>OR</tt>, and <tt>NOT</tt> operators.<br>"
                                "Examples: <br><tt>foo</tt><br><tt>bar*</tt><br><tt>fooANDbar*</tt><br>"),
                        size=40,
                    )),
                    ("exclude",
                    ListOfStrings(
                        title=_("Exclude tests by tag (<tt>--exclude</tt>)"),
                        help=_("Select test cases not to run by tag. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#tagging-test-cases\">About tagging test cases</a>)<br>These tests are"
                                " not run even if included with <tt>--include</tt>. <br>Tags are"
                                " matched using same rules as with <tt>--include</tt>.<br>"),                                                
                        size=40,
                    )),
                    ("critical",
                    ListOfStrings(
                        title=_("Critical test tag (<tt>--critical</tt>)"),
                        help=_("Tests having the given tag are considered critical. (<b>This is no threshold</b>, see <a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#setting-criticality\">About setting criticality</a>)<br>"
                                "If no critical tags are set, all tests are critical.<br>"
                                "Tags can be given as a pattern same way as with <tt>--include</tt>.<br>"),
                        size=40,
                    )),
                    ("noncritical",
                    ListOfStrings(
                        title=_("Non-Critical test tag (<tt>--noncritical</tt>)"),
                        help=_("Tests having the given tag are considered non-critical, even if also <tt>--critical</tt> is set. (<b>This is no threshold</b>, see <a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#setting-criticality\">About setting criticality</a>)<br>"
                                "Tags can be given as a pattern same way as with <tt>--include</tt>.<br>"),
                        size=40,
                    )),
                    ("variable",
                    ListOf(
                        Tuple(
                            elements=[
                                TextAscii(
                                    title=_("Variable name:")
                                ),
                                TextAscii(
                                    title=_("Value:"),
                                ),
                            ],
                            orientation="horizontal",
                        ),
                        movable=False,
                        title=_("Variables (<tt>--variable</tt>)"),
                        help=_("Set variables in the test data. <br>Only scalar variables with string"
                        " value are supported and name is given without <tt>${}</tt>. <br>"
                        " (See <tt>--variablefile</tt> for a more powerful variable setting mechanism.)<br>")
                    )),
                    ("variablefile",
                    ListOfStrings(
                        title=_("Load variables from file (<tt>--variablefile</tt>)"),
                        help=_("Python or YAML file file to read variables from. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#variable-files\">About variable files</a>)<br>Possible arguments to the variable file can be given"
                                " after the path using colon or semicolon as separator.<br>"
                                "Examples:<br> "                                               
                                "<tt>path/vars.yaml</tt><br>"
                                "<tt>set_environment.py:testing</tt><br>"),                                                
                        size=40,
                    )),
                    ("exitonfailure",
                    DropdownChoice(
                        title=_("Exit on failure (<tt>--exitonfailure</tt>)"),
                        help=_("Stops test execution if any critical test fails. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#stopping-when-first-test-case-fails\">About failed tests</a>)"),
                        choices=[
                            ('yes', _('yes')),
                            ('no', _('no')),
                        ],
                        default_value="no",
                    )),   
                    ("host",
                    MonitoredHostname(
                        title=_("Piggyback host"),
                        help=
                        _("Piggyback allows to assign the results of this particular Robot test to another host."),
                        allow_empty=False,
                    )),                                                                                     
                ],
            )
        ]),
        title=_("<b>Robot Framework test suites</b>"),
        help=_(helptext),
        add_label=_("Add test suite"),
        movable=True,
    )
)

# Section header for encoding: https://checkmk.de/check_mk-werks.php?werk_id=1425
# Available encodings: https://docs.python.org/2.4/lib/standard-encodings.html
dropdown_robotmk_output_encoding=CascadingDropdown(
    title=_("Agent output encoding"),
    help=_("This setting controls how the XML results of Robot Framework are encoded. <br>"
        " - <b>UTF-8</b>: for small to medium tests. Easy to read and debug (100% plain text). This is the default Checkmk setting.<br>"
        " - <b>BASE-64</b>: for a more condensed agent output. Saves line breaks (but <i>not</i> space).<br>"
        " - <b>Zlib</b>: for large results, compressed at maximum level (>95%). "
    ),
    choices=[
        ('utf_8', _('UTF-8')),
        ('base64_codec', _('BASE-64')),
        ('zlib_codec', _('Zlib (compressed)')),
    ],
    default_value="plain_utf8",
)



dropdown_robotmk_log_rotation=CascadingDropdown(
    title=_("Number of days to keep Robot log files on the host"),
    help=_("This settings helps to keep the test host clean by <b>deleting the log files</b> after a certain amount of days. Log files are: <br>"
    "<tt>robotframework-$SUITENAME-$timestamp-output.xml<br>"
    "<tt>robotframework-$SUITENAME-$timestamp-log.html<br>"
    "<tt>robotframework-$SUITENAME-$timestamp-report.html<br>"
    ),
    choices=[
        ('0', _('0 (always delete last log)')),
        ('1', _('1')),
        ('3', _('3')),
        ('7', _('7')),
        ('14', _('14')),
        ('30', _('30')),
        ('90', _('90')),
        ('365', _('365')),
        ('never', _('Keep all log files')),
    ],
    default_value="14",
    sorted=False
)




dropdown_robotmk_execution_choices=CascadingDropdown(
    title=_("Type of execution"),
    choices=[
        ("cmk_async", _("Async mode: Agent executes RobotMK plugin"),
            Tuple(
            elements=[
                Dictionary(
                    help=_(helptext_execution_mode_cmk_async),
                    elements=[
                        agent_config_cache_time_cmk_async,
                        agent_config_robotdir,
                        agent_config_listof_testsuites(helptext=helptext_listof_testsuites_cmkasync),  
                    ]
                )
            ]
            )
        ),
        ("external_spooldir", _("Spooldir mode: RobotMK plugin executed externally"),
            Tuple(
            elements=[
                Dictionary(
                    help=_(helptext_execution_mode_spooldir),
                    elements=[
                        agent_config_cache_time_spooldir,
                        agent_config_robotdir,
                        agent_config_listof_testsuites(helptext=helptext_listof_testsuites_spooldir),  
                    ]
                )
            ]
            )
        ),
    ]
)


def _valuespec_agent_config_robotmk():
    return Alternative(
        title=_("RobotMK (Linux, Windows)"),
        help=_(
            "This will deploy the agent plugin to execute Robot Framework E2E test on the remote host "
            "and a .YML configuration file with the list of test suites to execute."),
        style="dropdown",
        elements=[
            Dictionary(
                title=_("Deploy the RobotMK plugin"),
                elements=[
                    ("execution_mode",
                    dropdown_robotmk_execution_choices
                    ),
                    ("agent_output_encoding",
                    dropdown_robotmk_output_encoding
                    ),
                    ("log_rotation",
                    dropdown_robotmk_log_rotation
                    ),
                ],
                optional_keys=["auth_instances"],
            ),
            FixedValue(
                None,
                title=_("Do not deploy the RobotMK plugin"),
                totext=_("(No Robot Framework tests on this machine)"),
            ),   
        ])


rulespec_registry.register(
    HostRulespec(
        group=RulespecGroupMonitoringAgentsAgentPlugins,
        name="agent_config:robotmk",
        valuespec=_valuespec_agent_config_robotmk,
    ))

def _valuespec_inventory_robotmk_rules():
    return Dictionary(
        title=_("Robot Framework Service Discovery"),
        
        elements=[
            ("robot_discovery_settings",
                ListOf(
                    Tuple(elements=[
                        TextAscii(
                            title=("Root suite pattern"),
                            allow_empty=True,
                            size=25,
                            default_value=".*",
                            help=_("Define a regular expression for the root suite in the Robot result you want to set the <b>discovery level</b>. To find out the root suite name, open output.xml of the Robot test and search for the very first suite tag.")
                        ),
                        DropdownChoice(
                            title = ("Level"),
                            choices = [
                                ( "0"  , _("0 - create one service from the top result element")),
                                ( "1"  , _("1 - create service(s) from each result element 1 level deeper")),
                                ( "2"  , _("2 - create service(s) from each result element 2 levels deeper")),
                                ( "3"  , _("3 - create service(s) from each result element 3 levels deeper")),
                            ],
                            help=_(
                                u"Each Robot result consists of one suite which is either the "
                                u".robot file name or the folder name containg the tests.<br>"
                                u"By default, RobotMK creates 1 service from this single root node.<br>"
                                u"Choosing another level enables you to <b>split the Robot result</b> into as many services as you want.<br>"
                                u"This is perfect for <b>suites</b> and <b>tests</b>. Even if possible, you should <i>not</i> create services from <b>keywords</b>!"
                            ),        
                        ),
                        TextAscii(
                            title=("Node Blacklist"),
                            allow_empty=True,
                            size=25,
                            default_value="",
                            help=_("By default, RobotMK will create services for <i>all</i> nodes on the discovery level. A <b>blacklist</b> pattern selectively hinders RobotMK to inventorize certain services.<br>"
                                "Note: An empty string is interpreted as an empty blacklist = inventorize all (default).")
                        ),                        
                    ]),  #Tuple
                    title=_("Discovery level of services from Robot output"),

                ) # ListOf
            ), 
            ("robot_service_prefix",
                ListOf(
                    Tuple(elements=[
                        TextAscii(
                            title=("Root suite pattern"),
                            allow_empty=True,
                            size=25,
                            default_value=".*",
                            help=_("Define a regular expression for the root suite in the Robot result you want to set the <b>service name prefix</b>. To find out the root suite name, open output.xml of the Robot test and search for the very first suite tag.")
                        ),

                        TextAscii(
                            title=("Service name prefix"),
                            allow_empty=True,
                            size=25,
                            default_value="Robot%SPACE%",
                            help=_("How Robot service names should start. If there should be a whitespace between prefix and name, mask it with <tt>%SPACE%</tt>.")
                        ),                              
                    ]),  #Tuple_elements
                    title=_("Service prefix for discovered Robot services"),

                ) # ListOf
            ), 
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

def _item_spec_robotmk():
    return TextAscii(title=_("Services"),
                     help=_("Matches the service names generated from <u>Robot suites</u>. By default this is always the <i>topmost</i> suite (level 0) which results in <i>one service</i>.<br> "
                            "Robot suites can be nested; to define a lower level CMK should "
                            "generate services from, use the service discovery rule "
                            "<i>Robot Framework Service Discovery</i>.<br>"))


dropdown_robotmk_show_submessages=CascadingDropdown(
    title=_("Show the messages of sub-nodes"),
        help=_("By default, suites and tests do not show messages of sub-items to save space. Depending on the suite it can make sense to activate this setting to get a more descriptive output line."),
        choices=[
            ('yes', _('yes')),
            ('no', _('no')),
        ],
        default_value="no",
)

 

def _parameter_valuespec_robotmk():
    return Dictionary(elements=[
        ("output_depth", Dictionary(  # L1 
            title = _('Output depth'),
            help = _('In Robot, suites and keywords can be nested. The default of RobotMK is to dissolve/recurse all nested objects and to show them in the service output.<br> '
                     'This is good in general, but sometimes not what you want (think of a keyword which is defined by five layers of abstraction).<br>'
                     'To keep the RobotMK output clear and understandable, set a proper pattern and e.g. <i>output depth=0</i> for sub-suites or keywords which should not get dissolved any deeper.<br>'
                     '(Hint: This is only for visual control; suites/keywords which are hidden by this setting can still be compared to <i>runtime_threshold</i> and change the overall suite state.)<br>'
                     'Patterns always start at the beginning.'
                     ),
            elements = [        
                ("output_depth_suites", ListOf(  # /L2
                    Tuple(  # L3
                        title = _('<b>Suite</b> Output depth'),
                        show_titles=True,
                        orientation="horizontal",
                        elements = [
                            TextAscii(
                                title=("<b>Suite</b> pattern"),
                                allow_empty=False,
                                size=60,
                            ),
                            Integer(
                                title=("depth"),
                                allow_empty=False,
                                size=3,
                            ),                            
                        ],
                    ),  # L3 / Tuple
                    add_label=_("Add"),
                    movable=False,
                    title=_("<b>Suite</b> Output depth")
                )), # L2 / output_depth_suites 
                ("output_depth_keywords", ListOf(  # /L2
                    Tuple(  # L3
                        title = _('<b>Keyword</b> Output depth'),
                        show_titles=True,
                        orientation="horizontal",
                        elements = [
                            TextAscii(
                                title=("<b>Keyword</b> pattern"),
                                allow_empty=False,
                                size=60,
                            ),
                            Integer(
                                title=("depth"),
                                allow_empty=False,
                                size=3,
                            ),                            
                        ],
                    ),  # L3 / Tuple
                    add_label=_("Add"),
                    movable=False,
                    title=_("<b>Keyword</b> Output depth")
                )), # L2 / output_depth_suites                                                   
            ],
        )), # L1 / output_depth   



        ("runtime_threshold", Dictionary(
            title = _('Runtime thresholds'),
            help = _('Define patterns here to assign runtime thresholds to suites, tests and keywords. <br>'
                    'A runtime exceedance always results in a WARN state and is propagated to the overall suite status.<br>'
                    'Always keep in mind that runtime monitoring is not a feature of Robot but RobotMK. This means that a Robot suite can have an internal OK state but WARN in CheckMK.<br>'
                    'Patterns always start at the beginning. CRIT threshold must be bigger than WARN; values of 0 disable the threshold.'
            ),
            elements = [
                ("runtime_threshold_suites", ListOf(  # /L2
                    Tuple(  # L3
                        title = _('<b>Suite</b> thresholds'),
                        show_titles=True,
                        orientation="horizontal",
                        elements = [
                            TextAscii(
                                title=("<b>Suite</b> pattern"),
                                allow_empty=False,
                                size=60,
                            ),
                            Float(
                                title=("WARN threshold (sec)"),
                                allow_empty=False,
                                size=19,
                            ),                            
                            Float(
                                title=("CRIT threshold (sec)"),
                                allow_empty=False,
                                size=19,
                            ),                            
                        ],
                    ),  # L3 / Tuple
                    add_label=_("Add"),
                    movable=False,
                    title=_("<b>Suite</b> thresholds")
                )), # L2 
               ("runtime_threshold_tests", ListOf(  # /L2
                    Tuple(  # L3
                        title = _('<b>Test</b> thresholds'),
                        show_titles=True,
                        orientation="horizontal",
                        elements = [
                            TextAscii(
                                title=("<b>Test</b> pattern"),
                                allow_empty=False,
                                size=60,
                            ),
                            Float(
                                title=("WARN threshold (sec)"),
                                allow_empty=False,
                                size=19,
                            ),                            
                            Float(
                                title=("CRIT threshold (sec)"),
                                allow_empty=False,
                                size=19,
                            ),                            
                        ],
                    ),  # L3 / Tuple
                    add_label=_("Add"),
                    movable=False,
                    title=_("<b>Test</b> thresholds")
                )), # L2                 
                ("runtime_threshold_keywords", ListOf(  # /L2
                    Tuple(  # L3
                        title = _('<b>Keyword</b> thresholds'),
                        show_titles=True,
                        orientation="horizontal",
                        elements = [
                            TextAscii(
                                title=("<b>Keyword</b> pattern"),
                                allow_empty=False,
                                size=60,
                            ),
                            Float(
                                title=("WARN threshold (sec)"),
                                allow_empty=False,
                                size=19,
                            ),                            
                            Float(
                                title=("CRIT threshold (sec)"),
                                allow_empty=False,
                                size=19,
                            ),                            
                        ],
                    ),  # L3 / Tuple
                    add_label=_("Add"),
                    movable=False,
                    title=_("<b>Keyword</b> thresholds")
                )), # L2                                       
            ],
        )), # L1 / runtime_threshold  

        ("perfdata_creation", Dictionary(
            title = _('Perfdata creation'),
            help = _('By default, no performance data are generated. Define patterns here to select suites, tests and keywords which should be displayed in graphs. <br>'
                    'Patterns always start at the beginning.'),
            elements = [
                ("perfdata_creation_suites", ListOfStrings(  # /L2
                    title = _('<b>Suite</b> perfdata'),
                    orientation="horizontal",
                    allow_empty=False,
                    size=60,
                    
                )), # L2 

               ("perfdata_creation_tests", ListOfStrings(  # /L2
                    title = _('<b>Test</b> perfdata'),
                    orientation="horizontal",
                    allow_empty=False,
                    size=60,
                )), # L2                
                ("perfdata_creation_keywords", ListOfStrings(  # /L2
                    title = _('<b>Keyword</b> perfdata'),
                    orientation="horizontal",
                    allow_empty=False,
                    size=60,
                )), # L2                                         
            ],
        )), # L1 / perfdata_creation         
        ("includedate", DropdownChoice(
            title=_("Include execution date in first output line"),
            help=_("If checked, the first output line of the check will also contain the timestamp when the suite was finished."),
            choices=[
                ('yes', _('yes')),
                ('no', _('no')),
            ],
            default_value="no",
        )),               
        ("show_submessages", dropdown_robotmk_show_submessages),     
    ],)

rulespec_registry.register(
    CheckParameterRulespecWithItem(
        check_group_name="robotmk",
        # gui/plugins/wato/utils/__init__.py
        group=RulespecGroupCheckParametersApplications,
        item_spec=_item_spec_robotmk,
        parameter_valuespec=_parameter_valuespec_robotmk,
        title=lambda: _("Robot Framework"),
    ))
