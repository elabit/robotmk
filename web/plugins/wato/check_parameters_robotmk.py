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

# TODO: Add logging True/False
# TODO: warn/crit threholds for total_runtime
# TODO: timeout nicht mehr automatisch von executoin int. berechnen lassen

#   _           _                   
#  | |         | |                  
#  | |__   __ _| | _____ _ __ _   _ 
#  | '_ \ / _` | |/ / _ \ '__| | | |
#  | |_) | (_| |   <  __/ |  | |_| |
#  |_.__/ \__,_|_|\_\___|_|   \__, |
#                              __/ |
#                             |___/ 



# EXECUTION MODE Help Texts --------------------------------
helptext_execution_mode_agent_serial="""
    The Checkmk agent starts the Robotmk plugin frequently (=<i>agent check interval</i>) in '<b>controller mode</b>'.<br>
    The controller then can start the plugin in '<b>runner mode</b>' if the <i>global suites execution interval</i> is over to execute suites in series.<br>
    After each suite, the runner writes the suite result data into a state file. <br>
    The controller does not wait for the runner to finish the execution of all suites; it reads the most recent state files of all configured suites and generates the agent output on STDOUT. <br>
    <b>Use cases</b> for this mode: in general, all Robot tests which can run headless and do not require a certain OS user."""
helptext_execution_mode_agent_parallel="""
    The Checkmk agent starts the Robotmk plugin frequently (=<i>agent check interval</i>) in '<b>controller mode</b>'.<br>
    For each suite, the controller reads the individual <i>suite execution interval</i> and decides whether to start a dedicated plugin process in '<b>runner mode</b>', parametrized with the suite's name.<br>
    Each runner writes its suite result into a state file. <br>
    The controller does not wait for the runner processes to finish; it reads the most recent state files of all configured suites and generates the agent output to print it on STDOUT.<br> 
    <b>Use cases</b> for this mode: same as '<i>agent_serial</i>' - in addition, this mode makes sense on test clients which have the CPU/Mem resources for parallel test execution."""
helptext_execution_mode_agent_parallel="This is only a placeholder for the parallel execution of RF suites. <b>Please choose another mode.</b>" 
helptext_execution_mode_external="""
    The Checkmk agent starts the Robotmk plugin frequently (=<i>agent check interval</i>) in '<b>controller mode</b>'.<br>
    The controller's only job in this mode is to read the most recent state files of all configured suites and generates the agent output to print it on STDOUT.<br> <br>
    You must use an external tool (e.g. cron/task scheduler) to execute Robot suites by starting the Robotmk runner with <tt>robotmk.py --run [SUITES]</tt>. <br>
    Each runner writes its suite result into a state file. <br>
    <tt>SUITES</tt> are suite IDs defined in <i>Robot Framework test suites</i> below. <br>
    If no suites are specified, the runner will execute all suites in <tt>robotmk.yml</tt>.<br>
    If there are no suites defined at all, the runner will execute all suites in the <i>Robot suites directory</i>. <br><br>   
    <b>Use cases</b> for this mode: <br>
      - Applications which need a desktop<br>
      - Applications which require to be run with a certain user account<br>
      - The need for more control about when to execute a Robot test and when not"""




# GLOBAL EXECUTION INTERVAL: only serial ===========================================================
agent_config_global_suites_execution_interval_agent_serial=Age(
        title=_("Runner <b>execution interval</b>"),
        help=_("Sets the interval in which the Robotmk <b>controller</b> will trigger the Robotmk <b>runner</b> to execute <b>all suites in series</b>.<br>"
        "The default is 15min but strongly depends on the maximum probable runtime of all <i>test suites</i>.<br>Choose an interval which is a good comprimise between frequency and execution runtime headroom.<br>"),
        minvalue=1,
        maxvalue=65535,
        default_value=900,
)

# GLOBAL CACHE TIME: serial & external =============================================================
agent_config_global_cache_time_agent_serial=Age(
        title=_("Result <b>cache time</b>"),
        help=_("Suite state files are updated by the <b>runner</b> after each execution (<i>global suites execution interval</i>).<br>"
        "The <b>controller</b> monitors the age of those files and expects them to be not older than the <i>global cache time</i>. <br>"
        "Each suite with a state file older than its <i>cache time</i> will be reported as 'stale'.<br>"
        "For obvious reasons, the cache time must always be set higher than the execution interval."),
        minvalue=1,
        maxvalue=65535,
        default_value=960,
)
agent_config_global_cache_time_external=Age(
    title=_("Global suite cache time"),
    help=_("Suite state files are updated every time when the <b>runner</b> has executed the suites.<br>"
    "The <b>controller</b> monitors the age of those files and expects them to be not older than the <i>global cache time</i> or the <i>suite cache time</i> (if set). <br>"
    "Each suite with a state file older than its <i>cache time</i> will be reported as 'stale'.<br>"
    "For obvious reasons, this cache time must always be set higher than the execution interval."),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

# SUITE CACHE TIMES: parallel & external ===========================================================
agent_config_suite_suites_cache_time_agent_parallel=Age(
    title=_("Suite cache time"),
    help=_("Sets the <b>suite specific</b> cache time. (Must be higher than the <i>suite execution interval</i>)"),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

agent_config_suite_suites_cache_time_external=Age(
    title=_("Suite cache time"),
    help=_("Sets <b>suite specific cache times</b> for <b>individual execution intervals</b>"),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

# SUITE EXECUTION INTERVAL: only parallel ==========================================================
agent_config_suite_suites_execution_interval_agent_parallel=Age(
    title=_("Suite execution interval"),
    help=_("Sets the interval in which the Robotmk <b>controller</b> will trigger the Robotmk <b>runner</b> to execute <b>this particular suite</b>.<br>"),
    minvalue=1,
    maxvalue=65535,
    default_value=900,
)

agent_config_testsuites_tag=Dictionary(
    title=_("Suite tag"),
    elements=[    
        ("tag",
        TextUnicode(
            help=_("Suites which are <b>added multiple times</b> (to execute them with different parameters) should have a <b>unique tag</b>.<br>"),
            allow_empty=False,
            size=30
        )),                                                                                             
    ]
)

agent_config_dict_robotdir = Dictionary(
    title=_("Change <b>Robot suites directory</b>"),
    elements=[
        ("robotdir",
        TextUnicode(
            help=_("By default the Robotmk plugin will search for Robot suites in the following paths:<br>"
                    " - <tt>/usr/lib/check_mk_agent/robot</tt> (Linux)<br>"
                    " - <tt>C:\\ProgramData\\checkmk\\agent\\robot</tt> (Windows) <br>"
                    "You can override this setting if Robot tests are stored in another path. <br>"
                    "Windows paths can be given with single backslashes; OS dependent path validation is made during the agent baking.<br>"),
            allow_empty=False,
            size=100,
            default_value=""
        ))
    ]
)    
 

agent_config_testsuites_piggybackhost=Dictionary(
    title=_("Piggyback host"),
    elements=[    
        ("piggybackhost",
        MonitoredHostname(
            help=_("Piggyback allows to assign the results of this particular Robot test to another host."),
            allow_empty=False,
        )),                                                                                             
    ]
)


agent_config_testsuites_path=TextUnicode(
    title=_("Robot test path"),
    help=_("Name of the <tt>.robot</tt> file or directory containing <tt>.robot</tt> files, relative to the <i>robot suites directory</i><br>"
        "It is highly recommended to organize Robot suites in <i>directories</i> and to specify the directories here without leading/trailing (back)slashes.<br>"
        ),
    allow_empty=False,
    size=50,
)

def gen_agent_config_testsuites_paramsdict(): 
    dict_elements=[    
        ("piggybackhost",
        MonitoredHostname(
            title=_("SPiggyback host"),
            help=_("Piggyback allows to assign the results of this particular Robot test to another host."),
            allow_empty=False,
        )),                                                                                             
    ]
    return Dictionary(
        elements=dict_elements
    )


agent_config_testsuites_robotframework_params_dict=Dictionary(
    help=_("The options here allow to specify the most common cmdline parameters for Robot Framework. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#all-command-line-options\">All command line options</a>)"),
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
    ],
)

agent_config_testsuites_robotframework_params_container=Dictionary(
    title=_("Robot Framework parameters"),
    elements=[    
        ("robot_params",
        agent_config_testsuites_robotframework_params_dict),
    ]
)


# Make the help text of SuitList dependent on the type of execution
def gen_agent_config_dict_listof_testsuites(mode):
    titledict = {
        'agent_serial': 'to execute with one runner',
        'agent_parallel': 'to execute individually',
        'external': 'to be executed externally'
    }
    return Dictionary (
        title=_("Specify <b>suites</b> " + titledict[mode]),
        elements=[(
            "suites",
            ListOf(
                gen_testsuite_tuple(mode),
                help=_("""
                    Click on '<i>Add test suite</i>' to add the suites to the execution list and to specify additional parameters, piggyback host and execution order. <br>
                    If you do not add any suite here, the Robotmk plugin will add every <tt>.robot</tt> file/every directory within the <i>Robot suites directory</i> to the execution list - without any further parametrization.<br>"""
                ),
                add_label=_("Add test suite"),
                movable=True,
            )
        )]
    )


def gen_testsuite_tuple(mode): 
    if mode =='agent_serial':
        return Tuple(elements=[
            agent_config_testsuites_path, 
            agent_config_testsuites_tag,
            agent_config_testsuites_piggybackhost,
            agent_config_testsuites_robotframework_params_container,
            # timing settings (there aren't any - set globally)
        ])
    if mode =='agent_parallel':
        return Tuple(elements=[
            agent_config_testsuites_path, 
            agent_config_testsuites_tag,
            agent_config_testsuites_piggybackhost,
            agent_config_testsuites_robotframework_params_container,
            # timing settings
            agent_config_suite_suites_cache_time_agent_parallel,
            agent_config_suite_suites_execution_interval_agent_parallel,
        ])
    if mode =='external':
        return Tuple(elements=[
            agent_config_testsuites_path, 
            agent_config_testsuites_tag,
            agent_config_testsuites_piggybackhost,
            agent_config_testsuites_robotframework_params_container,
            # timing settings
            agent_config_suite_suites_cache_time_external,
        ])

dropdown_robotmk_output_encoding=CascadingDropdown(
    title=_("Agent output encoding"),
    help=_("""
        The agent payload of Robotmk is JSON with fields for <b>XML and HTML data</b> (which can contain embedded images). <br>
        To save bandwidth and resources, this fields are by default <b>zlib compressed</b> to 5% of their size.<br>
        Unless you are debugging or curious there should be no reason to change the encoding."""
    ),
    choices=[
        ('zlib_codec', _('Zlib compressed')),
        ('utf_8', _('UTF-8')),
        ('base64_codec', _('BASE-64')),
    ],
    default_value="zlib_codec",
)

dropdown_robotmk_transmit_html=DropdownChoice(
    title=_("Transmit HTML log"),
    help=_("""
    Besides the XML data, the Robotmk plugin also transmits the <b>HTML log file</b> written by Robot Framework to the Checkmk server.\n
    You can disable the HTML log transmission if you do not have a need for this kind of logs on the server.
    """
    ),
    choices=[
        ( False,  _("No") ),
        ( True, _("Yes") ),
    ],
    default_value = True,
)

dropdown_robotmk_log_rotation=CascadingDropdown(
    title=_("Number of days to keep Robot XML/HTML log files on the host"),
    help=_("This settings helps to keep the test host clean by <b>deleting the log files</b> after a certain amount of days. Log files are: <br>"
    "<tt>robotframework-$SUITENAME-$timestamp-output.xml<br>"
    "<tt>robotframework-$SUITENAME-$timestamp-log.html<br>"
    ),
    choices=[
        ('0', _('0 (keep only the last logfile)')),
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

dropdown_robotmk_logging=DropdownChoice(
    title=_("Robotmk logging"),
    help=_("""
    By default, the Robotmk plugin writes all steps and decisions into <tt>robotmk.log</tt>."""
    ),
    choices=[
        ( False,  _("No") ),
        ( True, _("Yes") ),
    ],
    default_value = True,
)

dropdown_robotmk_execution_choices=CascadingDropdown(
    title=_("Execution mode"),
    help=_(
            "The <b>execution mode</b> is a general setting which controls who runs RF suites, how and when.<br>"
            "For this, the Robotmk plugin operates in two modes:  '<b>controller</b>' and '<b>runner</b>'.<br><br>"
            "<b>controller</b> mode: <br>"
            "- active when executed with no cmdline arguments by agent<br>"
            "- creates a list of suites as defined in either the YML file, or by environment variables<br>"
            "- calls itself in runner mode (-> suite execution)<br>"
            "- reads the written state files, monitors them for staleness<br>"
            "- writes output to STDOUT for the CMK agent<br><br>"
            "<b>runner</b> mode: <br>"
            "- runs the suites given as arguments ro <tt>--run</tt>; if no suites are given, it runs all defined. <br>"
            "- writes suite state files <br><br>"
            "The behaviour of both modes depends on the execution mode you set here."
    ),
    sorted=False,
    choices=[
        ("agent_serial", _("agent_serial"),
            Tuple(
            help=_(helptext_execution_mode_agent_serial),
            elements=[
                gen_agent_config_dict_listof_testsuites("agent_serial"),  
                agent_config_global_cache_time_agent_serial,
                agent_config_global_suites_execution_interval_agent_serial,
            ]
            )
        ),
        ("agent_parallel", _("agent_parallel (no yet implemented)"),
            Tuple(
            help=_(helptext_execution_mode_agent_parallel),
            elements=[
                gen_agent_config_dict_listof_testsuites("agent_parallel"),  
            ]
            )
        ),
        ("external", _("external"),
            Tuple(
            help=_(helptext_execution_mode_external),
            elements=[
                gen_agent_config_dict_listof_testsuites("external"),  
                agent_config_global_cache_time_external,
            ]
            )
        ),
    ]
)


def _valuespec_agent_config_robotmk():
    return Alternative(
        title=_("Robotmk (Linux, Windows)"),
        help=_(
            "This rule will deploy the <b>Robotmk agent plugin</b> and a generated YML config file (<tt>robotmk.yml</tt>) to the remote host."),
        style="dropdown",
        elements=[
            Dictionary(
                title=_("Deploy the Robotmk plugin"),
                elements=[
                    ("robotdir", 
                    agent_config_dict_robotdir
                    ),
                    # agent_serial, agent_parallel, external
                    ("execution_mode",
                    dropdown_robotmk_execution_choices
                    ),
                    ("agent_output_encoding",
                    dropdown_robotmk_output_encoding
                    ),
                    ("transmit_html",
                    dropdown_robotmk_transmit_html
                    ),
                    ("logging",
                    dropdown_robotmk_logging
                    ),
                    ("log_rotation",
                    dropdown_robotmk_log_rotation
                    ),
                ],
                optional_keys=["auth_instances"],
            ),
            FixedValue(
                None,
                title=_("Do not deploy the Robotmk plugin"),
                totext=_("(No Robot Framework tests on this machine)"),
            ),   
        ])


rulespec_registry.register(
    HostRulespec(
        group=RulespecGroupMonitoringAgentsAgentPlugins,
        name="agent_config:robotmk",
        valuespec=_valuespec_agent_config_robotmk,
    ))



#       _ _                                   
#      | (_)                                  
#    __| |_ ___  ___ _____   _____ _ __ _   _ 
#   / _` | / __|/ __/ _ \ \ / / _ \ '__| | | |
#  | (_| | \__ \ (_| (_) \ V /  __/ |  | |_| |
#   \__,_|_|___/\___\___/ \_/ \___|_|   \__, |
#                                        __/ |
#                                       |___/ 

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
                            help=_("By default, Robotmk will create services for <i>all</i> nodes on the discovery level. A <b>blacklist</b> pattern selectively hinders Robotmk to inventorize certain services.<br>"
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
                            size=60,
                            default_value="Robot E2E $SUITENAME $TAG$SPACE",
                            help=_("""
                                How Robot service names of discovered items should start. The following Variables can be used (usage: <tt>$VAR</tt> or <tt>${VAR}</tt>):<br>
                                <tt>${PATH}</tt>  -  Name of Robot suite directory or <tt>.robot</tt> file<br>
                                <tt>${SUITENAME}</tt>  -  Name of top level suite (usually same name as path)<br>
                                <tt>${TAG}</tt>  -  Suite tag<br>
                                <tt>${SUITEID}</tt>  -  short for <tt>${PATH}_${TAG}</tt><br>
                                <tt>${EXEC_MODE}</tt>  -  Execution mode<br>
                                <tt>${SPACE}</tt>  -  Use this if there should be a space between the prefix and the item name<br><br>
                                The default format string is "<tt>Robot Framework E2E $SUITEID$SPACE-$SPACE</tt>".
                            """)
                        ),                              
                    ]),  #Tuple_elements
                    title=_("Naming rules for discovered Robot services"),

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

listof_runtime_threshold_suites=ListOf(  
    Tuple(  
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
    ), 
    add_label=_("Add"),
    movable=False,
    title=_("<b>Suite</b> thresholds")
)

listof_runtime_threshold_tests=ListOf(  
    Tuple(  
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
    ),  
    add_label=_("Add"),
    movable=False,
    title=_("<b>Test</b> thresholds")
)

listof_runtime_threshold_keywords=ListOf(  
    Tuple(  
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
)

dropdown_robotmk_show_all_runtimes=CascadingDropdown(
    title=_("Show monitored runtimes also when in OK state"),
        help=_("By default, Robotmk only displays the runtime of Robot suites/tests/keywords where a threshold was exceeded. This helps to keep the output much cleaner. <br> "
            "To baseline newly created Robot tests for a certain time, it can be helpful to show even OK runtime values."),
        choices=[
            ('yes', _('yes')),
            ('no', _('no')),
        ],
        default_value="no",
)

#        _               _    
#       | |             | |   
#    ___| |__   ___  ___| | __
#   / __| '_ \ / _ \/ __| |/ /
#  | (__| | | |  __/ (__|   < 
#   \___|_| |_|\___|\___|_|\_\
                            
                             

def _parameter_valuespec_robotmk():
    return Dictionary(elements=[
        ("output_depth", Dictionary(  # L1 
            title = _('Output depth'),
            help = _('In Robot, suites and keywords can be nested. The default of Robotmk is to dissolve/recurse all nested objects and to show them in the service output.<br> '
                     'This is good in general, but sometimes not what you want (think of a keyword which is defined by five layers of abstraction).<br>'
                     'To keep the Robotmk output clear and understandable, set a proper pattern and e.g. <i>output depth=0</i> for sub-suites or keywords which should not get dissolved any deeper.<br>'
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
                    'Always keep in mind that runtime monitoring is not a feature of Robot but Robotmk. This means that a Robot suite can have an internal OK state but WARN in CheckMK.<br>'
                    'Patterns always start at the beginning. CRIT threshold must be bigger than WARN; values of 0 disable the threshold.'
            ),
            elements = [                
                ("runtime_threshold_suites", listof_runtime_threshold_suites),   
                ("runtime_threshold_tests", listof_runtime_threshold_tests),   
                ("runtime_threshold_keywords", listof_runtime_threshold_keywords),   
                ("show_all_runtimes", dropdown_robotmk_show_all_runtimes),                                      
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
