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
                               Tuple, CascadingDropdown)

from cmk.gui.plugins.wato import (CheckParameterRulespecWithItem,
                                  rulespec_registry,
                                  RulespecGroupCheckParametersDiscovery,
                                  HostRulespec)
try:
    # V2
    from cmk.gui.cee.plugins.wato.agent_bakery.rulespecs.utils import RulespecGroupMonitoringAgentsAgentPlugins
except ImportError:
    # V1.6
    from cmk.gui.cee.plugins.wato.agent_bakery import RulespecGroupMonitoringAgentsAgentPlugins


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
helptext_execution_mode_agent_serial = """
    The Checkmk agent starts the Robotmk <b>controller</b> as a <i>synchronous</i> check plugin in the <i>agent check interval</i>.<br>
    It also starts the Robotmk <b>runner</b> as an <i>asynchronous</i> check plugin in the <i>runner execution interval</i>.<br>
    If you do not specify suites, the runner will execute all suites in the <i>Robot suites directory</i>. <br><br>
    <b>Use cases</b> for this mode:<br>
    In general, all Robot tests which can run headless and do not require a certain OS user."""
helptext_execution_mode_agent_parallel = """(not yet implemented)"""
# The Checkmk agent starts the Robotmk <b>controller</b> as a normal check plugin (= in <i>agent check interval</i>).<br>
# For each suite, the controller reads the individual <i>suite execution interval</i> and decides whether to start a dedicated plugin process in '<b>runner mode</b>', parametrized with the suite's name.<br>
# Each runner writes its suite result into a state file. <br>
# The controller does not wait for the runner processes to finish; it reads the most recent state files of all configured suites and generates the agent output to print it on STDOUT.<br>
# <b>Use cases</b> for this mode: same as '<i>agent_serial</i>' - in addition, this mode makes sense on test clients which have the CPU/Mem resources for parallel test execution."""
helptext_execution_mode_agent_parallel = "This is only a placeholder for the parallel execution of RF suites. <b>Please choose another mode.</b>"
helptext_execution_mode_external = """
    The Checkmk agent starts the Robotmk <b>controller</b> as a <i>synchronous</i> check plugin in the <i>agent check interval</i>.<br>
    <b>Rule dependency</b>: The rule <i>Deploy custom files with agent</i> (package <tt>robotmk-external</tt>) places the <b>runner</b> within the agent's <tt>bin</tt> directory. 
    From there, you can start the runner with any external tool (e.g. systemd timer/cron/task scheduler).<br><br>
    If no suites are specified, the runner will execute all suites listed in <tt>robotmk.yml</tt>.<br>
    If no suites are defined at all, the runner will execute all suites found in the <i>Robot suites directory</i>. <br><br>   
    <b>Use cases</b> for this mode: <br>
      - Desktop Applications<br>
      - Applications which require to be run with a certain user account (SSO)<br>
      - The need for more control about when to execute a Robot test and when not"""

# GLOBAL EXECUTION INTERVAL: only serial ===========================================================
agent_config_global_suites_execution_interval_agent_serial = Age(
    title=_("Runner <b>execution interval</b>"),
    help=
    _("Interval the Checkmk agent will execute the <b>runner</b> plugin asynchronously.<br>"
      "The default is 15min but strongly depends on the maximum probable runtime of all <i>test suites</i>.<br>Choose an interval which is a good comprimise between frequency and execution runtime headroom.<br>"
      ),
    minvalue=1,
    maxvalue=65535,
    default_value=900,
)

# GLOBAL CACHE TIME: serial & external =============================================================
agent_config_global_cache_time_agent_serial = Age(
    title=_("Result <b>cache time</b>"),
    help=
    _("Suite state files are updated by the <b>runner</b> after each execution (<i>Runner execution interval</i>).<br>"
      "The <b>controller</b> monitors the age of those files and expects them to be not older than the <i>global cache time</i>. <br>"
      "Each suite with a state file older than its <i>result cache time</i> will be reported as 'stale'.<br>"
      "For obvious reasons, the cache time must always be set higher than the <i>runner execution interval</i>."
      ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)
agent_config_global_cache_time_external = Age(
    title=_("Global suite cache time"),
    help=
    _("Suite state files are updated every time when the <b>runner</b> has executed the suites.<br>"
      "The <b>controller</b> monitors the age of those files and expects them to be not older than the <i>global cache time</i> or the <i>suite cache time</i> (if set). <br>"
      "Each suite with a state file older than its <i>cache time</i> will be reported as 'stale'.<br>"
      "For obvious reasons, this cache time must always be set higher than the execution interval."
      ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

# SUITE CACHE TIMES: parallel & external ===========================================================
agent_config_suite_suites_cache_time_agent_parallel = Age(
    title=_("Suite cache time"),
    help=
    _("Sets the <b>suite specific</b> cache time. (Must be higher than the <i>suite execution interval</i>)"
      ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

agent_config_suite_suites_cache_time_external = Age(
    title=_("Suite cache time"),
    help=
    _("Sets <b>suite specific cache times</b> for <b>individual execution intervals</b>"
      ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

# SUITE EXECUTION INTERVAL: only parallel ==========================================================
agent_config_suite_suites_execution_interval_agent_parallel = Age(
    title=_("Suite execution interval"),
    help=
    _("Sets the interval in which the Robotmk <b>controller</b> will trigger the Robotmk <b>runner</b> to execute <b>this particular suite</b>.<br>"
      ),
    minvalue=1,
    maxvalue=65535,
    default_value=900,
)

agent_config_testsuites_tag = Dictionary(
    title=_("Suite tag"),
    elements=[
        ("tag",
         TextUnicode(help=_(
             "Suites which are <b>added multiple times</b> (to execute them with different parameters) should have a <b>unique tag</b>.<br>"
         ),
                     allow_empty=False,
                     size=30)),
    ])

agent_config_dict_robotdir = Dictionary(
    title=_("Change <b>Robot suites directory</b>"),
    elements=
    [("robotdir",
      TextUnicode(help=_(
          "By default the Robotmk plugin will search for Robot suites in the following paths:<br>"
          " - <tt>/usr/lib/check_mk_agent/robot</tt> (Linux)<br>"
          " - <tt>C:\\ProgramData\\checkmk\\agent\\robot</tt> (Windows) <br>"
          "You can override this setting if Robot tests are stored in another path. <br>"
          "Windows paths can be given with single backslashes; OS dependent path validation is made during the agent baking.<br>"
      ),
                  allow_empty=False,
                  size=100,
                  default_value=""))])

agent_config_testsuites_piggybackhost = Dictionary(
    title=_("Piggyback host"),
    elements=[
        ("piggybackhost",
         MonitoredHostname(
            help=
             _("Piggyback allows to assign the results of this particular Robot test to another host."
               ),
         )),
    ])

agent_config_testsuites_path = TextUnicode(
    title=_("Robot test path"),
    help=
    _("Name of the <tt>.robot</tt> file or directory containing <tt>.robot</tt> files, relative to the <i>robot suites directory</i><br>"
      "It is highly recommended to organize Robot suites in <i>directories</i> and to specify the directories here without leading/trailing (back)slashes.<br>"
      ),
    allow_empty=False,
    size=50,
)

agent_config_testsuites_robotframework_params_dict = Dictionary(
    help=
    _("The options here allow to specify the most common cmdline parameters for Robot Framework. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#all-command-line-options\">All command line options</a>)"
      ),
    elements=[
        ("name",
         TextUnicode(
             title=_("Top level suite name (<tt>--name</tt>)"),
             help=
             _("Set the name of the top level suite. By default the name is created based on the executed file or directory.<br>"
               "This sets the name of a fresh discovered Robot service; an already existing service will hide away and will be found by the discovery under a new name."
               ),
             allow_empty=False,
             size=50,
         )),
        ("suite",
         ListOfStrings(
             title=_("Select suites (<tt>--suite</tt>)"),
             help=
             _("Select suites by name. <br>When this option is used with"
               " <tt>--test</tt>, <tt>--include</tt> or <tt>--exclude</tt>, only tests in"
               " matching suites and also matching other filtering"
               " criteria are selected. <br>"
               " Name can be a simple pattern similarly as with <tt>--test</tt> and it can contain parent"
               " name separated with a dot. <br>"
               " For example, <tt>X.Y</tt> selects suite <tt>Y</tt> only if its parent is <tt>X</tt>.<br>"
               ),
             size=40,
         )),
        ("test",
         ListOfStrings(
             title=_("Select test (<tt>--test</tt>)"),
             help=
             _("Select tests by name or by long name containing also"
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
             help=
             _("Select tests by tag. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#tagging-test-cases\">About tagging test cases</a>)<br>Similarly as name with <tt>--test</tt>,"
               "tag is case and space insensitive and it is possible"
               "to use patterns with <tt>*</tt>, <tt>?</tt> and <tt>[]</tt> as wildcards.<br>"
               "Tags and patterns can also be combined together with"
               "<tt>AND</tt>, <tt>OR</tt>, and <tt>NOT</tt> operators.<br>"
               "Examples: <br><tt>foo</tt><br><tt>bar*</tt><br><tt>fooANDbar*</tt><br>"
               ),
             size=40,
         )),
        ("exclude",
         ListOfStrings(
             title=_("Exclude tests by tag (<tt>--exclude</tt>)"),
             help=
             _("Select test cases not to run by tag. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#tagging-test-cases\">About tagging test cases</a>)<br>These tests are"
               " not run even if included with <tt>--include</tt>. <br>Tags are"
               " matched using same rules as with <tt>--include</tt>.<br>"),
             size=40,
         )),
        ("critical",
         ListOfStrings(
             title=_("Critical test tag (<tt>--critical</tt>)"),
             help=
             _("Tests having the given tag are considered critical. (<b>This is no threshold</b>, see <a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#setting-criticality\">About setting criticality</a>)<br>"
               "If no critical tags are set, all tests are critical.<br>"
               "Tags can be given as a pattern same way as with <tt>--include</tt>.<br>"
               ),
             size=40,
         )),
        ("noncritical",
         ListOfStrings(
             title=_("Non-Critical test tag (<tt>--noncritical</tt>)"),
             help=
             _("Tests having the given tag are considered non-critical, even if also <tt>--critical</tt> is set. (<b>This is no threshold</b>, see <a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#setting-criticality\">About setting criticality</a>)<br>"
               "Tags can be given as a pattern same way as with <tt>--include</tt>.<br>"
               ),
             size=40,
         )),
        ("variable",
         ListOf(
             Tuple(
                 elements=[
                     TextAscii(title=_("Variable name:")),
                     TextAscii(title=_("Value:"), ),
                 ],
                 orientation="horizontal",
             ),
             movable=False,
             title=_("Variables (<tt>--variable</tt>)"),
             help=
             _("Set variables in the test data. <br>Only scalar variables with string"
               " value are supported and name is given without <tt>${}</tt>. <br>"
               " (See <tt>--variablefile</tt> for a more powerful variable setting mechanism.)<br>"
               ))),
        ("variablefile",
         ListOfStrings(
             title=_("Load variables from file (<tt>--variablefile</tt>)"),
             help=_(
                 "Python or YAML file file to read variables from. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#variable-files\">About variable files</a>)<br>Possible arguments to the variable file can be given"
                 " after the path using colon or semicolon as separator.<br>"
                 "Examples:<br> "
                 "<tt>path/vars.yaml</tt><br>"
                 "<tt>set_environment.py:testing</tt><br>"),
             size=40,
         )),
        ("exitonfailure",
         DropdownChoice(
             title=_("Exit on failure (<tt>--exitonfailure</tt>)"),
             help=
             _("Stops test execution if any critical test fails. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#stopping-when-first-test-case-fails\">About failed tests</a>)"
               ),
             choices=[
                 ('yes', _('yes')),
                 ('no', _('no')),
             ],
             default_value="no",
         )),
    ],
)

agent_config_testsuites_robotframework_params_container = Dictionary(
    title=_("Robot Framework parameters"),
    elements=[
        ("robot_params", agent_config_testsuites_robotframework_params_dict),
    ])


# Make the help text of SuitList dependent on the type of execution
def gen_agent_config_dict_listof_testsuites(mode):
    titledict = {
        'agent_serial': 'to execute with one runner',
        'agent_parallel': 'to execute individually',
        'external': 'to be executed externally'
    }
    return Dictionary(title=_("Specify <b>suites</b> " + titledict[mode]),
                      elements=[("suites",
                                 ListOf(
                                     gen_testsuite_tuple(mode),
                                     help=_("""
                    Click on '<i>Add test suite</i>' to add the suites to the execution list and to specify additional parameters, piggyback host and execution order. <br>
                    If you do not add any suite here, the Robotmk plugin will add every <tt>.robot</tt> file/every directory within the <i>Robot suites directory</i> to the execution list - without any further parametrization.<br>"""
                                            ),
                                     add_label=_("Add test suite"),
                                     movable=True,
                                 ))])


def gen_testsuite_tuple(mode):
    if mode == 'agent_serial':
        return Tuple(elements=[
            agent_config_testsuites_path,
            agent_config_testsuites_tag,
            agent_config_testsuites_piggybackhost,
            agent_config_testsuites_robotframework_params_container,
            # timing settings (there aren't any - set globally)
        ])
    if mode == 'agent_parallel':
        return Tuple(elements=[
            agent_config_testsuites_path,
            agent_config_testsuites_tag,
            agent_config_testsuites_piggybackhost,
            agent_config_testsuites_robotframework_params_container,
            # timing settings
            agent_config_suite_suites_cache_time_agent_parallel,
            agent_config_suite_suites_execution_interval_agent_parallel,
        ])
    if mode == 'external':
        return Tuple(elements=[
            agent_config_testsuites_path,
            agent_config_testsuites_tag,
            agent_config_testsuites_piggybackhost,
            agent_config_testsuites_robotframework_params_container,
            # timing settings
            # agent_config_suite_suites_cache_time_external,
        ])


dropdown_robotmk_output_encoding = CascadingDropdown(
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

dropdown_robotmk_transmit_html = DropdownChoice(
    title=_("Transmit HTML log"),
    help=_("""
    Besides the XML data, the Robotmk plugin also transmits the <b>HTML log file</b> written by Robot Framework to the Checkmk server.\n
    You can disable the HTML log transmission if you do not have a need for this kind of logs on the server.
    """),
    choices=[
        (False, _("No")),
        (True, _("Yes")),
    ],
    default_value=True,
)

dropdown_robotmk_log_rotation = CascadingDropdown(
    title=_("Number of days to keep Robot XML/HTML log files on the host"),
    help=_(
        "This settings helps to keep the test host clean by <b>deleting the log files</b> after a certain amount of days. Log files are: <br>"
        "<tt>robotframework-$SUITENAME-$timestamp-output.xml<br>"
        "<tt>robotframework-$SUITENAME-$timestamp-log.html<br>"),
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
    sorted=False)

dropdown_robotmk_logging = DropdownChoice(
    title=_("Robotmk logging"),
    help=_("""
    By default, the Robotmk plugin writes all steps and decisions into <tt>robotmk.log</tt>."""
           ),
    choices=[
        (False, _("No")),
        (True, _("Yes")),
    ],
    default_value=True,
)

dropdown_robotmk_execution_choices = CascadingDropdown(
    title=_("Execution mode"),
    help=
    _("The <b>execution mode</b> is a general setting which controls who runs RF suites, how and when.<br>"
      "For this, Robotmk comes with two agent scripts:<br><br>"
      "<tt>robotmk.py</tt> - the '<b>controller</b>':<br>"
      "- determines the configured suites<br>"
      "- reads their JSON state files<br>"
      "- writes all JSON objects to STDOUT for the CMK agent<br><br>"
      "<tt>robotmk-runner.py</tt> - the '<b>runner</b>':<br>"
      "- determines the configured suites<br>"
      "- runs the suites<br>"
      "- collects suite logs and writes their JSON state files <br><br>"
      "The behaviour and usage of both scripts depends on the execution mode you set here.<br>"
      "<b>Rule dependency:</b> All modes require the rule '<i>Limit script types to execute</i>' to allow the execution of <tt>.py</tt> files. "
      ),
    sorted=False,
    choices=[
        ("agent_serial", _("agent_serial"),
         Tuple(help=_(helptext_execution_mode_agent_serial),
               elements=[
                   gen_agent_config_dict_listof_testsuites("agent_serial"),
                   agent_config_global_cache_time_agent_serial,
                   agent_config_global_suites_execution_interval_agent_serial,
               ])),
        # ("agent_parallel", _("agent_parallel (no yet implemented)"),
        #  Tuple(help=_(helptext_execution_mode_agent_parallel),
        #        elements=[
        #            gen_agent_config_dict_listof_testsuites("agent_parallel"),
        #        ])),
        ("external", _("external"),
         Tuple(help=_(helptext_execution_mode_external),
               elements=[
                   gen_agent_config_dict_listof_testsuites("external"),
                   agent_config_global_cache_time_external,
               ])),
    ])


def _valuespec_agent_config_robotmk():
    return Alternative(
        title=_("Robotmk (Linux, Windows)"),
        help=
        _("Robotmk integrates the results of <b>Robot Framework</b> tests into Checkmk. This rule will deploy the <b>Robotmk agent plugin</b> and a generated YML control file (<tt>robotmk.yml</tt>) to the remote host."
          ),
        style="dropdown",
        elements=[
            Dictionary(
                title=_("Deploy the Robotmk plugin"),
                elements=[
                    ("robotdir", agent_config_dict_robotdir),
                    # agent_serial, agent_parallel, external
                    ("execution_mode", dropdown_robotmk_execution_choices),
                    ("agent_output_encoding",
                     dropdown_robotmk_output_encoding),
                    ("transmit_html", dropdown_robotmk_transmit_html),
                    ("logging", dropdown_robotmk_logging),
                    ("log_rotation", dropdown_robotmk_log_rotation),
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
