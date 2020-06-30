#!/usr/bin/python
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


def _valuespec_agent_config_robotmk():
    return Alternative(
        title=_("RobotMK (Linux, Windows)"),
        help=_(
            "This will deploy the agent plugin to execute Robot Framework E2E test on the remote host "
            "and a .YML configuration file with the list of test suites to execute."),
        style="dropdown",
        elements=[
            Dictionary(title=_("Deploy the RobotMK plugin"),
                       #optional_keys=["runas"],
                       elements=[
                           ("cache_time",
                            Age(
                                title=_("Agent Plugin Cache time / execution interval"),
                                help=_("Set a custom interval the RobotMK plugin should be executed on the host (instead of normal check interval)"),
                                minvalue=1,
                                maxvalue=65535,
                                default_value=900,
                            )),
                            # Leave this commented. Outputdir is not something the user should need to change. 
                            # ("outputdir",
                            # TextUnicode(
                            #     # regex="^[-a-zA-Z0-9._]*$",
                            #     regex_error=_("Your outputdir has an invalid format."),
                            #     title=_("Output directory of where XML test result is stored"),
                            #     help=_("If nothing is filled out, the default will be used. Default assumes a linux path. Path validation is made during baking."),
                            #     allow_empty=True,
                            #     default_value="/tmp/"
                            # )),
                            ("robotdir",
                            TextUnicode(
                                title=_("Robot suites directory"),
                                help=_("By default the RobotMK plugin will search for Robot suites in <tt>/usr/lib/check_mk_agent/robot</tt> (Linux) or <tt>C:\\ProgramData\\checkmk\\agent\\robot</tt> (Windows). <br>"
                                        "Windows paths can be given with single backslashes; OS dependent path validation is made during baking.<br>"),
                                allow_empty=True,
                                size=100,
                                default_value=""
                            )), 
                            ("suites",
                            ListOf(
                                Tuple(elements=[
                                    TextUnicode(
                                        title=_("Robot test file/dir name"),
                                        help=_("Robot Framework can execute files (.robot) as well as nested directories "
                                            "which itself contain .robot files. All names are expected to be relative to the robot dir."),
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
                                            # FIXME: test includes!
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
                                            # FIXME: test excludes!
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
                                            ("variables",
                                            ListOfStrings(
                                                title=_("Variables (<tt>--variable</tt>)"),
                                                help=_("Set variables in the test data. <br>Only scalar variables with string"
                                                " value are supported and name is given without <tt>${}</tt>. <br>"
                                                " See <tt>--variablefile</tt> for a more powerful variable setting mechanism.<br>"
                                                "Example: <tt>varname:value</tt><br>"),
                                                # size=600,
                                                orientation="vertical",
                                                valuespec=TextUnicode(
                                                    size=40,
                                                    regex=".*:.*",
                                                    regex_error=_("Please enter a key-value pair separated by ':'"),
                                                ),
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

                                            # ("variables",
                                            # ListOf(
                                            #     Tuple(elements=[
                                            #         TextUnicode(
                                            #             title=_("Key"),
                                            #             allow_empty=False,
                                            #             help=_("The key")
                                            #         ),
                                            #         TextUnicode(
                                            #             title=_("Value"),
                                            #             allow_empty=False,
                                            #             help=_("The value")
                                            #         ),
                                                    
                                            #     ]),
                                            #     title=_("Variables"),
                                            #     help=_("This must form a key:value pair"),
                                            # )),
                                            # TODO
                                            # Replace by Tuple, e.g. 
                                            #Tuple(elements=[
                                            #    Filesize(title=_("Warning below")),
                                            #    Filesize(title=_("Critical below"))
                                            # ],)),

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
                                            # Proposal: piggybackhost instead of host                   
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
                                title=_("Test suites"),
                                help=
                                _("Click <i>Add test suite</i> to add Robot suites to the execution and drag them to re-order. <br>"
                                "Each test suite will be executed after the other.<br>"),
                                add_label=_("Add test suite"),
                                movable=True,
                            )), # test suites, Listof
                       ]),
            FixedValue(
                None,
                title=_("Do not deploy the RobotMK plugin"),
                totext=_("(disabled)"),
            ),
        ],
    )

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
            ("robot_discovery_level",
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
                                u"Choosing another level enables you to <b>split the Robot result</b> into "
                                u"as many services as you want.<br>"
                                u"Keep in mind that the deeper you choose this level, the more likely "
                                u"it is that you will also get services out from tests and keywords (if this is what you want...)."
                            ),        
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
                            default_value="Robot ",
                            help=_("How Robot service names should start")
                        ),                              
                    ]),  #Tuple_elements
                    title=_("Service prefix for discovered Robot services"),

                ) # ListOf
            ), 
        ],  # elements
    )


# def _valuespec_inventory_robotmk_rules():
#     return Dictionary(
#         title=_("Robot Framework Service Discovery"),
        
#         elements=[
#             ("robot_discovery",
#                 ListOf(
#                     Tuple(elements=[

#                         TextAscii(
#                             title=("Service name prefix"),
#                             allow_empty=True,
#                             size=25,
#                             default_value="Robot ",
#                             help=_("How all Robot service names start")
#                         ),                        
#                         Dictionary(
#                             title=_("Discovery level"),
#                             elements=[
#                                 ( "discovery_level",
#                                     DropdownChoice(
#                                         choices = [
#                                             ( "0"  , _("0 - create ONE service from the one result element in the top level")),
#                                             ( "1"  , _("1 - create service(s) from each result element 1 level deeper")),
#                                             ( "2"  , _("2 - create service(s) from each result element 2 levels deeper")),
#                                             ( "3"  , _("3 - create service(s) from each result element 3 levels deeper")),
#                                         ],
#                                     )
#                                 ),
#                             ],
#                             help=_(
#                                 u"Each Robot result consists of one suite which is either the "
#                                 u".robot file name or the folder name containg the tests.<br>"
#                                 u"By default, RobotMK creates 1 service from this single root node.<br>"
#                                 u"Choosing another level enables you to split the Robot result into "
#                                 u"as many services as you want.<br>"
#                                 u"Keep in mind that the deeper you choose this level, the more likely "
#                                 u"it is that you will also get services out from tests and keywords (if this is what you want...)."
#                                 ),                                                    
#                         ),       
#                         TextAscii(
#                             title=("Root suite pattern"),
#                             allow_empty=True,
#                             size=25,
#                             default_value=".*",
#                             help=_("A regular expression matching the single root node of the Robot result.")
#                         ),
#                     ]),  #Tuple_elements
#                     title=_("Adopt discovery of services from Robot output"),

#                 ) # ListOf
#             ), 
#         ],  # elements
#     )



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
                    'Patterns always start at the beginning.'
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
                                title=("WARN threshold (s)"),
                                allow_empty=False,
                                size=6,
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
                                title=("WARN threshold (s)"),
                                allow_empty=False,
                                size=6,
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
                                title=("WARN threshold (s)"),
                                allow_empty=False,
                                size=6,
                            ),                            
                        ],
                    ),  # L3 / Tuple
                    add_label=_("Add"),
                    movable=False,
                    title=_("<b>Keyword</b> thresholds")
                )), # L2                                       
            ],
        )), # L1 / runtime_threshold  


        # TODO: Helper function comma delimited ??
        ("perfdata_creation", Dictionary(
            title = _('Perfdata creation'),
            help = _('By default, no performance data are generated. Define patterns here to select suites, tests and keywords which should be displayed in graphs. <br>'
                    'Patterns always start at the beginning.'),
            elements = [
                ("perfdata_creation_suites", ListOfStrings(  # /L2
                    title = _('<b>Suite</b> perfdata'),
                    orientation="horizontal",
                    size=60,
                    
                )), # L2 

               ("perfdata_creation_tests", ListOfStrings(  # /L2
                    title = _('<b>Test</b> perfdata'),
                    orientation="horizontal",
                    size=60,
                )), # L2                
                ("perfdata_creation_keywords", ListOfStrings(  # /L2
                    title = _('<b>Keyword</b> perfdata'),
                    orientation="horizontal",
                    size=60,
                )), # L2                                         
            ],
        )), # L1 / perfdata_creation                          
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
