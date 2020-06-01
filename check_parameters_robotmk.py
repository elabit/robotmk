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
            "This will deploy and configure the Check_MK agent plugin <tt>mk_sap_hana</tt>. "
            "To make this plugin work you have to configure USERSTOREKEY or USER and PASSWORD, "
            "ie. USERSTOREKEY=SVAMON and SID=I08 means we need a key for SVAMONI08 in the HDB "
            "userstore specified in $MK_CONFDIR/sap_hana.cfg. Moreover you can configure "
            "'RUNAS' with the following values 'agent' or 'instance'. The latter one is default. "
            " Use the FQDN in the query if HOSTNAME is not set, other the short hostname."),
        style="dropdown",
        elements=[
            Dictionary(title=_("Deploy the RobotMK plugin"),
                       #optional_keys=["runas"],
                       elements=[
                           ("cache_time",
                            Age(
                                title=_("Cache time of data"),
                                minvalue=1,
                                maxvalue=65535,
                                default_value=30,
                            )),
                            ("test_suites",
                            ListOf(
                                Dictionary(
                                    optional_keys=["piggyhost"],
                                    elements=[
                                        ("piggyhost",
                                        Hostname(
                                            title=_("Monitoring host this test suite should be mapped to"),
                                            help=
                                            _("If you leave this empty then the test suite will run on the host "
                                                "where the <tt>robotmk</tt> plugin is running. In this case the "
                                                "name of all executed suites must be unique."),
                                        )),
                                        ("outputdir",
                                        TextAscii(
                                            regex="^[-a-zA-Z0-9._]*$",
                                            regex_error=_("Your outputdir has an invalid format."),
                                            title=_("Output directory of where XML test result is stored"),
                                            help=_("If nothing is filled out, the default will be used"),
                                            allow_empty=True,
                                            default_value="OMD_ROOT"
                                        )),
                                        ("robotdir",
                                        TextAscii(
                                            regex="^[-a-zA-Z0-9._]*$",
                                            regex_error=_("Your output dir has an invalid format."),
                                            help=_("If nothing is filled out, the default will be used"),
                                            title=_("The directory where the robot suites are living"),
                                            allow_empty=True,
                                            default_value="OMD_ROOT"
                                        )),
                                        ("tags",
                                        ListOfStrings(
                                            title=_("tags to go through"),
                                            help=_("The tags matching what will be executed :)"),
                                            size=40,
                                        )),
                                        ("variables",
                                        ListOfStrings(
                                            title=_("variables to use"),
                                            help=_("Only scalar are supported. Must be supplied as key/value pair"),
                                            size=40,
                                            orientation="vertical",
                                            valuespec=TextAscii(
                                                size=20,
                                                regex=".*:.*",
                                                regex_error=_("Please entere a key-value pair separated by ':'"),
                                            ),
                                        )),
                                        ("dry_run",
                                        Checkbox(
                                            title=_("Dry run this test suite"),
                                            label=_("Do a dry run instead of actually running the test !"),
                                        )),
                                    ],
                                ),
                                title=_("Test suites"),
                                help=
                                _("Inspired by the ORACLE monitoring rule :)"),
                                add_label=_("Add test suite"),
                                movable=False,
                            )),
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
                        ( "0"  , _("Directory level 0 (one service with root folder name)") ),
                        ( "1"  , _("Directory level 1 (n services for each test file/subfolder") ),
                        ( "2"  , _("Directory level 2 (\"-\")") ),
                        ( "3"  , _("Directory level 3 (\"-\")") ),
                    ]
            )),
            # TODO: Service Prefix
        ],
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
