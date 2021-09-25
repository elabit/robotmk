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
                               Tuple, CascadingDropdown, Integer, Transform)

from cmk.gui.plugins.wato import (CheckParameterRulespecWithItem,
                                  rulespec_registry,
                                  RulespecGroupCheckParametersDiscovery,
                                  HostRulespec)

from cmk.gui.log import logger   

try:
    # V2
    from cmk.gui.cee.plugins.wato.agent_bakery.rulespecs.utils import RulespecGroupMonitoringAgentsAgentPlugins
except ImportError:
    # V1.6
    from cmk.gui.cee.plugins.wato.agent_bakery import RulespecGroupMonitoringAgentsAgentPlugins



#   _           _
#  | |         | |
#  | |__   __ _| | _____ _ __ _   _
#  | '_ \ / _` | |/ / _ \ '__| | | |
#  | |_) | (_| |   <  __/ |  | |_| |
#  |_.__/ \__,_|_|\_\___|_|   \__, |
#                              __/ |
#                             |___/

# This dict only adds the new key only if 
# * the key already exists
# * the value is a boolean in fact 
# * the value contains something meaningful
# This prevents that empty dicts are set as values.
class DictNoNone(dict):
    def __setitem__(self, key, value):
        if (key in self or type(value) is bool) or bool(value):
            dict.__setitem__(self, key, value)

# Ref d3vh2I
# This class will be used as a helper for the Transform class. 
# The methods forth/back are planned as constructors for the instance and will 
# transform the data in the needed way. 
class RMKConfig():
    def __init__(self):
        self._cfg_dict = {
            'global': DictNoNone(),
            'suites': DictNoNone(),
        }        

    @property
    def as_canonical_dict(self):
        """Returns the RMK Config as the canonical dictionary"""
        logger.critical("ASDICT -------")
        logger.critical(self._cfg_dict)
        return self._cfg_dict


    @classmethod
    def wato_back(cls, data):
        """Convert the data structure coming from WATO and return the RMK dict"""
        # logger.critical("WATO BACK -------")
        # logger.critical(data)
        # rmk_config = RMKConfig()
        # rmk_config._cfg_dict = data
        
        # rmk_config.execution_mode = data['execution_mode'][0]
        # rmk_config.agent_output_encoding = data['agent_output_encoding']
        # rmk_config.transmit_html = data['transmit_html']
        # rmk_config.logging = data['logging']
        # rmk_config.log_rotation = data['log_rotation']
        # rmk_config.robotdir = data['dirs'].get('robotdir', None)
        # rmk_config.outputdir = data['dirs'].get('outputdir', None)
        return data
        return rmk_config.as_canonical_dict

    @classmethod
    def wato_forth(cls, data):
        """Convert the canonical data structure coming from the rule to present in WATO"""
        # logger.critical("WATO FORTH -------")
        # logger.critical(data)
        # See Ref YEZDRT which demonstrates a new WATO field.
        # The forth here checks if it is present in the loaded data and adds it, if not.
        # if not 'transmit_html1' in data: 
        #     data['transmit_html1'] = True
        # rmk_config = RMKConfig()
        # rmk_config._cfg_dict = data
        # logger.critical(rmk_config.execution_mode)
        # logger.critical(rmk_config.agent_output_encoding)
        # logger.critical(rmk_config.transmit_html)
        # logger.critical(rmk_config.logging)
        # logger.critical(rmk_config.log_rotation)
        # logger.critical(rmk_config.robotdir)
        # logger.critical(rmk_config.outputdir)
        return data

    @classmethod
    def from_env(cls):
        """Creates the RMK Config from environment variables (TBD)"""
        rmk_config = RMKConfig()
        return rmk_config._conf


    @property
    def global_dict(self):
        return self._cfg_dict['global']

    @property
    def suites_dict(self):
        return self.cfg_dict['suites']
    # ------------------------
    @property
    def execution_mode(self):
        return self.global_dict['execution_mode']

    @execution_mode.setter
    def execution_mode(self, val):
        self.global_dict['execution_mode'] = val
    # ------------------------
    @property
    def agent_output_encoding(self):
        return self.global_dict['agent_output_encoding']

    @agent_output_encoding.setter
    def agent_output_encoding(self, val):
        self.global_dict['agent_output_encoding'] = val
    # ------------------------
    @property
    def transmit_html(self):
        return self.global_dict['transmit_html']

    @transmit_html.setter
    def transmit_html(self, val):
        self.global_dict['transmit_html'] = val
    # ------------------------
    @property
    def logging(self):
        return self.global_dict['logging']

    @logging.setter
    def logging(self, val):
        self.global_dict['logging'] = val
    # ------------------------
    @property
    def log_rotation(self):
        return self.global_dict['log_rotation']

    @log_rotation.setter
    def log_rotation(self, val):
        self.global_dict['log_rotation'] = val
    # ------------------------
    @property
    def robotdir(self):
        return self.global_dict['robotdir']

    @robotdir.setter
    def robotdir(self, val):
        self.global_dict['robotdir'] = val
    # ------------------------
    @property
    def outputdir(self):
        return self.global_dict['outputdir']

    @outputdir.setter
    def outputdir(self, val):
        self.global_dict['outputdir'] = val




# EXECUTION MODE Help Texts --------------------------------
helptext_execution_mode_agent_serial = """
    The Checkmk agent starts the Robotmk <b>controller</b> as a <i>synchronous</i> check plugin in the <i>agent check interval</i>.<br>
    Simultanously, the agent starts the Robotmk <b>runner</b> as an <i>asynchronous</i> check plugin in the <i>runner execution interval</i>.<br>
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
    The Checkmk agent starts the Robotmk <b>controller</b> as a <i>synchronous</i> check plugin in the <i>agent check interval</i>.<br><br>
    <b>Important note for Checkmk 1.6</b>: The rule <i>Deploy custom files with agent</i> (package <tt>robotmk-external</tt>) must be used to place the <b>runner</b> within the agent's <tt>bin</tt> directory (there is no other way in Checkmk 1 to deploy files to that folder).<br>
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
    _("This configures the interval in which the Checkmk agent will execute the <b>runner</b> plugin asynchronously.<br>"
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
      "For obvious reasons, the cache time must always be set higher than the <i>runner execution interval</i>, including reruns of failed tests/subsuites (if configured).<br>"
      "(Do not confuse it with the <i>cache time</i> which Checkmk uses for the agent plugin configuration.)"
      ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)
agent_config_global_cache_time_external = Age(
    title=_("Result <b>cache time</b>"),
    help=
    _("Suite state files are updated every time when the <b>runner</b> has executed the suites.<br>"
      "The <b>controller</b> monitors the age of those files and expects them to be not older than the <i>global cache time</i> or the <i>suite cache time</i> (if set). <br>"
      "Each suite with a state file older than its <i>cache time</i> will be reported as 'stale'.<br>"
      "For obvious reasons, this cache time must always be set higher than the execution interval, including reruns of failed tests/subsuites (if configured)."
      ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

# SUITE CACHE TIMES: parallel & external ===========================================================
agent_config_suite_suites_cache_time_agent_parallel = Age(
    title=_("Suite cache time"),
    help=
    _("Sets the <b>suite specific</b> cache time. (Must be higher than the <i>suite execution interval</i>, including reruns of failed tests/subsuites)"
      ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

agent_config_suite_suites_cache_time_external = Age(
    title=_("Suite cache time"),
    help=
    _("Sets <b>suite specific cache times</b> for <b>individual execution intervals, including reruns of failed tests/subsuites</b>"
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


agent_config_testsuites_tag = TextUnicode(
    title=_("Suite tag"),
    help=_("Suites which are <b>added multiple times</b> (to execute them with different parameters) should have a <b>unique tag</b>.<br>"),
    allow_empty=False,
    size=30)


agent_config_dict_dirs = Dictionary(
    title=_("Change <b>default directories</b>"),
    help=_("This settings allow to override paths where Robotmk stores files. "),
    elements=[
        ("robotdir", TextUnicode(
            help=_(
                "Defines where the Robotmk plugin will search for <b>Robot suites</b>. By default this is:<br>"
                " - <tt>/usr/lib/check_mk_agent/robot</tt> (Linux)<br>"
                " - <tt>C:\\ProgramData\\checkmk\\agent\\robot</tt> (Windows) <br>"
                "Windows paths can be given with single backslashes; OS dependent path validation is made during the agent baking.<br>"
            ),
            title=_("Robot suites directory (<tt>robotdir</tt>)"),
            allow_empty=False,
            size=100,
            default_value=""
            )
        ),
        ("outputdir", TextUnicode(
            help=_(
                "Defines where Robot Framework <b>XML/HTML</b> and the <b>Robotmk JSON state files</b> will be stored. By default this is:<br>"                
                " - <tt>/var/log/robotmk</tt> (Linux)<br>"
                " - <tt>C:\\ProgramData\\checkmk\\agent\\log\\robotmk</tt> (Windows) <br>"
                "Windows paths can be given with single backslashes; OS dependent path validation is made during the agent baking.<br>"
            ),
            title=_("Output directory"),
            allow_empty=False,
            size=100,
            default_value=""
            )
        ),
        ("logdir", TextUnicode(
            help=_(
                "Defines where Robotmk <b>controller/runner execution log files</b> will be written to. By default this is:<br>"
                " - <tt>/var/log/robotmk</tt> (Linux)<br>"
                " - <tt>C:\\ProgramData\\checkmk\\agent\\log\\robotmk</tt> (Windows) <br>"
                "Windows paths can be given with single backslashes; OS dependent path validation is made during the agent baking.<br>"
            ),
            title=_("Log directory"),
            allow_empty=False,
            size=100,
            default_value=""
            )
        ),
    ])

# TODO: how to limit to only 1 host?
agent_config_testsuites_piggybackhost = MonitoredHostname( 
    title=_("Piggyback host"),
    help=_("Piggyback allows to assign the results of this particular Robot test to another host."),
)

agent_config_testsuites_path = TextUnicode(
    title=_("Robot test path"),
    help=
    _("Name of the <tt>.robot</tt> file or directory containing <tt>.robot</tt> files, relative to the <i>robot suites directory</i><br>"
      "It is highly recommended to organize Robot suites in <i>directories</i> and to specify the directories here without leading/trailing (back)slashes.<br>"
      ),
    allow_empty=False,
    size=50,
)

# TEST SELECTION DICT ELEMENTS =================================================
# To be used in test selection and rerunfailed
# Ref: 7uBbn2
dict_el_suite_selection = (
    "suite",
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
        )
)
dict_el_test_selection = (
    "test",
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
        )
)
dict_el_test_include = (
    "include",
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
        )
)

dict_el_test_exclude = (
    "exclude",
        ListOfStrings(
            title=_("Exclude tests by tag (<tt>--exclude</tt>)"),
            help=
            _("Select test cases not to run by tag. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#tagging-test-cases\">About tagging test cases</a>)<br>These tests are"
            " not run even if included with <tt>--include</tt>. <br>Tags are"
            " matched using same rules as with <tt>--include</tt>.<br>"),
            size=40,
        )
)


dict_el_suite_argsfile = (
    "argumentfile",
        ListOfStrings(
            title=_("Load arguments from file (<tt>--argumentfile</tt>)"),
            help=
            _("Name of files containing <b>additional command line arguments</b> for Robot Framework. The paths are relative to the <i>robot suites directory</i>.<br>"
            "Argument files allow placing all or some command line options and arguments into an external file where they will be read. This is useful for more exotic RF parameters not natively supported by Robotmk or for problematic characters.<br>"
            "The arguments given here are taken into use along with possible other command line options. (See also <a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#argument-files\">About argument files</a>)<br><br>"),
            size=70,
        )
)

dict_el_suite_variablefile = (
    "variablefile",
        ListOfStrings(
            title=_("Load variables from file (<tt>--variablefile</tt>)"),
            help=_(
                "Python or YAML file file to read variables from. (<a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#variable-files\">About variable files</a>)<br>Possible arguments to the variable file can be given"
                " after the path using colon or semicolon as separator.<br>"
                "Examples:<br> "
                "<tt>path/vars.yaml</tt><br>"
                "<tt>set_environment.py:testing</tt><br>"),
            size=70,
        )
)

agent_config_testsuites_robotframework_params_dict = Dictionary(
    title=_("Robot Framework parameters"),
    help=_(
        "The options here allow to specify the most common <b>commandline parameters</b> for Robot Framework.<br>"
        "In order to use other parameters (see <a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#all-command-line-options\">All command line options</a>), you can use the option 'Load arguments from file'.<br> (Feel free to <a href=\"https://github.com/simonmeggle/robotmk/issues\">file an issue</a> if you think that a special parameter should be added here)"
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
         # Ref: 7uBbn2
        dict_el_suite_selection,
        dict_el_test_selection,
        dict_el_test_include,
        dict_el_test_exclude,
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
        dict_el_suite_variablefile,               
        # dict_el_suite_customargs,
        dict_el_suite_argsfile,
        ("exitonfailure",
         DropdownChoice(
             title=_("Exit on failure (<tt>--exitonfailure</tt>)"),
             help=_(
                 "By default, Robot Framework tries to execute <i>every</i> test. If a test fails, the remaining keywords are skipped and the next test starts (see <a href=\"https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#stopping-when-first-test-case-fails\">About failed tests</a>)<br>"
                 "With this option set to 'yes', Robot Framework will leave the suite execution after the first test has failed."
               ),
             choices=[
                 ('yes', _('yes')),
                 ('no', _('no')),
             ],
             default_value="no",
         )),
    ],
)

agent_config_testsuites_max_executions_selection_dict = Dictionary(
    help=_("""
    With the following options it is possible to further filter the list of tests/suites to re-run. (Documentation: <a href=\"http://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#re-executing-failed-test-cases\">Re-executing failed test cases</a>)
    """),
    title=_("Re-execution filter"),
    elements=[
        # Ref: 7uBbn2
        dict_el_suite_selection,
        dict_el_test_selection,
        dict_el_test_include,
        dict_el_test_exclude,
    ]
)



agent_config_testsuites_max_executions_tuple = Dictionary(
    title=_("Handling of failed tests/suites"),
    help=_("""
    Robotmk can <b>immediately re-execute failed tests</b> n-times before submitting the result to the agent. After the n-th iteration, the total suite result contains the most current result of each test case.<br>
    Use this only as a last resort, for example when applications behave unreliable. Also take into account that the re-execution of failed tests/suites requires additional headroom for the <i>result cache time</i>. 
    """),
    elements=[
            ("max_executions", Integer(
                title=_("Maximum executions"),
                help=_("The maximum number of iterations (including the first attempt)"),
                minvalue=1,
                default_value=2
            )),
            ("rerun_selection", agent_config_testsuites_max_executions_selection_dict),
    ],
    optional_keys=['rerun_selection'],
)



agent_config_testsuites_max_executions_container = Dictionary(
    title=_("Handling of failed tests/suites"),
    elements=[
        ("failed_handling", agent_config_testsuites_max_executions_tuple),
    ])



# Make the help text of SuitList dependent on the type of execution
def gen_agent_config_dict_listof_testsuites(mode):
    titledict = {
        'agent_serial': 'to execute',
        'agent_parallel': 'to execute individually',
        'external': 'to be executed externally'
    }
    return ListOf(
        gen_testsuite_tuple(mode),
        help=_("""
Click on '<i>Add test suite</i>' to specify the suites to be executed, including additional parameters, piggyback host and execution order. This is the recommended way.<br>
If you do not add any suite here, the Robotmk plugin will add every <tt>.robot</tt> file/every directory within the <i>Robot suites directory</i> to the execution list - without any further parametrization.<br>"""
            ),
        add_label=_("Add test suite"),
        movable=True,
        title=_("Suites"),
    )

def gen_testsuite_tuple(mode):
    if mode == 'agent_serial':
        return Dictionary(
            elements=[
                ("path", agent_config_testsuites_path),
                ("tag", agent_config_testsuites_tag),
                ("piggybackhost", agent_config_testsuites_piggybackhost),
                ("robot_params", agent_config_testsuites_robotframework_params_dict),
                ("failed_handling", agent_config_testsuites_max_executions_tuple),
            ],
            optional_keys=['tag', 'piggybackhost', 'robot_params', 'failed_handling']
        )

    if mode == 'external':
        return Dictionary(
            elements=[
                ("path", agent_config_testsuites_path),
                ("tag", agent_config_testsuites_tag),
                ("piggybackhost", agent_config_testsuites_piggybackhost),
                ("robot_params", agent_config_testsuites_robotframework_params_dict),
                ("failed_handling", agent_config_testsuites_max_executions_tuple),
            ],
            optional_keys=['tag', 'piggybackhost', 'robot_params', 'failed_handling']
        )


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
    title=_("Transmit HTML log to Checkmk server"),
    help=_("""
    Robotmk transmits the <b>HTML log file</b> written by Robot Framework to the Checkmk server, where it can be action-linked with the discovered services. <br>
    This feature needs some <b>configuration</b> which you can find in the <b>Robotmk discovery rule</b>, option <i>'Restrict the HTML log files link creation'</i>.
    """),
    choices=[
        (False, _("No")),
        (True, _("Yes")),
    ],
    default_value=False,
)

dropdown_robotmk_log_rotation = CascadingDropdown(
    title=_("Number of days to keep Robot XML/HTML log files on the host"),
    help=_(
        "This setting helps to keep the test host clean by <b>deleting the log files</b> after a certain amount of days. Log files are: <br>"
        "<tt>robotframework-$SUITENAME-$timestamp-output.xml<br>"
        "<tt>robotframework-$SUITENAME-$timestamp-log.html<br>"),
    choices=[
        (1, _('1')),
        (3, _('3')),
        (7, _('7')),
        (14, _('14')),
        (30, _('30')),
        (90, _('90')),
        (365, _('365')),
    ],
    default_value=14,
    sorted=False)

dropdown_robotmk_logging = DropdownChoice(
    title=_("Robotmk log level"),
    help=_("""
    By default, the Robotmk plugin writes a <b>log file</b> for the controller and runner plugin. You can set the <b>verbosity</b> here."""
           ),
    choices=[
        ("OFF", _("Off (No logging)")),
        ("CRITICAL", _("Critical (least verbose)")),        
        ("ERROR", _("Error")),
        ("WARNING", _("Warning")),
        ("INFO", _("Info")),
        ("DEBUG", _("Debug (most verbose)")),
    ],
    default_value="INFO",
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
        ("agent_serial", _("agent_serial"), Dictionary(
            help=_(helptext_execution_mode_agent_serial),
            optional_keys=False,
            elements=[
                ("suites", gen_agent_config_dict_listof_testsuites("agent_serial")),
                ("cache_time", agent_config_global_cache_time_agent_serial),
                ("execution_interval", agent_config_global_suites_execution_interval_agent_serial),
            ])),
        #  Tuple(help=_(helptext_execution_mode_agent_serial),
        #        elements=[
        #            gen_agent_config_dict_listof_testsuites("agent_serial"),
        #            agent_config_global_cache_time_agent_serial,
        #            agent_config_global_suites_execution_interval_agent_serial,
        #        ])),
        # ("agent_parallel", _("agent_parallel (no yet implemented)"),
        #  Tuple(help=_(helptext_execution_mode_agent_parallel),
        #        elements=[
        #            gen_agent_config_dict_listof_testsuites("agent_parallel"),
        #        ])),
        ("external", _("external"), Dictionary(
            help=_(helptext_execution_mode_external),
            optional_keys=False,
            elements=[
                ("suites", gen_agent_config_dict_listof_testsuites("external")),
                ("cache_time", agent_config_global_cache_time_external),
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
            Transform(
                Dictionary(
                    title=_("Deploy the Robotmk plugin"),
                    elements=[
                        # agent_serial, agent_parallel, external
                        ("execution_mode", dropdown_robotmk_execution_choices),
                        ("agent_output_encoding", dropdown_robotmk_output_encoding),
                        ("transmit_html", dropdown_robotmk_transmit_html),
                        # Ref YEZDRT (forth example)
                        # ("transmit_html1", dropdown_robotmk_transmit_html),
                        ("log_level", dropdown_robotmk_logging),
                        ("log_rotation", dropdown_robotmk_log_rotation),
                        ("dirs", agent_config_dict_dirs),
                    ],
                    optional_keys=False,
                ),
                # Ref d3vh2I
                # read from rules.mk, present in WATO
                forth = RMKConfig.wato_forth,
                # forth = lambda x: x,
                # read from WATO
                back = RMKConfig.wato_back
                # back = lambda x: x,
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
