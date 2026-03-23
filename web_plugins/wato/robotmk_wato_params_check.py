#!/usr/bin/python

# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

# Compatibility layer for Checkmk 2.5+ (v1 API) and older versions (legacy API)
try:
    # Checkmk 2.5+ uses v1 API
    from cmk.rulesets.v1 import Title, Help
    from cmk.rulesets.v1.form_specs import (
        DefaultValue,
        DictElement,
        Dictionary,
        Float as FloatV1,
        Integer,
        List,
        SingleChoice,
        SingleChoiceElement,
        String,
        BooleanChoice,
    )
    from cmk.rulesets.v1.rule_specs import CheckParameters, Topic, HostAndItemCondition
    USES_V1_API = True
except ImportError:
    USES_V1_API = False

# Only import legacy API if v1 is not available AND we're in a GUI context
if not USES_V1_API:
    try:
        from cmk.gui.i18n import _
        from cmk.gui.valuespec import (
            CascadingDropdown,
            Dictionary,
            DropdownChoice,
            Float,
            Integer,
            ListOf,
            ListOfStrings,
            TextAscii,
            Tuple,
        )
        from cmk.gui.plugins.wato import (
            CheckParameterRulespecWithItem,
            rulespec_registry,
            RulespecGroupCheckParametersApplications,
        )
        LEGACY_API_AVAILABLE = True
    except ImportError:
        # Running standalone without GUI context
        LEGACY_API_AVAILABLE = False

# =============================================================================
# V1 API Implementation (Checkmk 2.5+)
# =============================================================================
if USES_V1_API:
    def _parameter_form_robotmk():
        return Dictionary(
            elements={
                "output_depth": DictElement(
                    required=False,
                    parameter_form=Dictionary(
                        title=Title("Output depth"),
                        help_text=Help(
                            "In Robot, suites and keywords can be nested. The default of Robotmk is to dissolve/recurse all nested objects and to show them in the service output. "
                            "This is good in general, but sometimes not what you want (think of a keyword which is defined by five layers of abstraction). "
                            "To keep the Robotmk output clear and understandable, set a proper pattern and e.g. output depth=0 for sub-suites or keywords which should not get dissolved any deeper. "
                            "(Hint: This is only for visual control; suites/keywords which are hidden by this setting can still be compared to runtime_threshold and change the overall suite state.) "
                            "Patterns always start at the beginning."
                        ),
                        elements={
                            "output_depth_suites": DictElement(
                                required=False,
                                parameter_form=List(
                                    element_template=Dictionary(
                                        elements={
                                            "pattern": DictElement(
                                                required=True,
                                                parameter_form=String(
                                                    title=Title("Suite pattern"),
                                                ),
                                            ),
                                            "depth": DictElement(
                                                required=True,
                                                parameter_form=Integer(
                                                    title=Title("depth"),
                                                ),
                                            ),
                                        },
                                    ),
                                    title=Title("Suite Output depth"),
                                ),
                            ),
                            "output_depth_keywords": DictElement(
                                required=False,
                                parameter_form=List(
                                    element_template=Dictionary(
                                        elements={
                                            "pattern": DictElement(
                                                required=True,
                                                parameter_form=String(
                                                    title=Title("Keyword pattern"),
                                                ),
                                            ),
                                            "depth": DictElement(
                                                required=True,
                                                parameter_form=Integer(
                                                    title=Title("depth"),
                                                ),
                                            ),
                                        },
                                    ),
                                    title=Title("Keyword Output depth"),
                                ),
                            ),
                        },
                    ),
                ),
                "runtime_threshold": DictElement(
                    required=False,
                    parameter_form=Dictionary(
                        title=Title("Runtime thresholds"),
                        help_text=Help(
                            "Define patterns here to assign runtime thresholds to suites, tests and keywords. "
                            "A runtime exceedance always results in a WARN state and is propagated to the overall suite status. "
                            "Always keep in mind that runtime monitoring is not a feature of Robot Framework but Robotmk; a Robot suite can have an internal OK state but be WARN in Checkmk! "
                            "Patterns always start at the beginning. CRIT threshold must be bigger than WARN; values of 0 disable the threshold."
                        ),
                        elements={
                            "runtime_threshold_suites": DictElement(
                                required=False,
                                parameter_form=List(
                                    element_template=Dictionary(
                                        elements={
                                            "pattern": DictElement(
                                                required=True,
                                                parameter_form=String(
                                                    title=Title("Suite pattern"),
                                                ),
                                            ),
                                            "warn": DictElement(
                                                required=True,
                                                parameter_form=FloatV1(
                                                    title=Title("WARN threshold (sec)"),
                                                ),
                                            ),
                                            "crit": DictElement(
                                                required=True,
                                                parameter_form=FloatV1(
                                                    title=Title("CRIT threshold (sec)"),
                                                ),
                                            ),
                                        },
                                    ),
                                    title=Title("Suite thresholds"),
                                ),
                            ),
                            "runtime_threshold_tests": DictElement(
                                required=False,
                                parameter_form=List(
                                    element_template=Dictionary(
                                        elements={
                                            "pattern": DictElement(
                                                required=True,
                                                parameter_form=String(
                                                    title=Title("Test pattern"),
                                                ),
                                            ),
                                            "warn": DictElement(
                                                required=True,
                                                parameter_form=FloatV1(
                                                    title=Title("WARN threshold (sec)"),
                                                ),
                                            ),
                                            "crit": DictElement(
                                                required=True,
                                                parameter_form=FloatV1(
                                                    title=Title("CRIT threshold (sec)"),
                                                ),
                                            ),
                                        },
                                    ),
                                    title=Title("Test thresholds"),
                                ),
                            ),
                            "runtime_threshold_keywords": DictElement(
                                required=False,
                                parameter_form=List(
                                    element_template=Dictionary(
                                        elements={
                                            "pattern": DictElement(
                                                required=True,
                                                parameter_form=String(
                                                    title=Title("Keyword pattern"),
                                                ),
                                            ),
                                            "warn": DictElement(
                                                required=True,
                                                parameter_form=FloatV1(
                                                    title=Title("WARN threshold (sec)"),
                                                ),
                                            ),
                                            "crit": DictElement(
                                                required=True,
                                                parameter_form=FloatV1(
                                                    title=Title("CRIT threshold (sec)"),
                                                ),
                                            ),
                                        },
                                    ),
                                    title=Title("Keyword thresholds"),
                                ),
                            ),
                            "show_all_runtimes": DictElement(
                                required=False,
                                parameter_form=SingleChoice(
                                    title=Title("Show monitored runtimes also when in OK state"),
                                    help_text=Help(
                                        "By default, Robotmk only displays the runtime of Robot suites/tests/keywords where a threshold was exceeded. This helps to keep the output much cleaner. "
                                        "To baseline newly created Robot tests for a certain time, it can be helpful to show even OK runtime values."
                                    ),
                                    elements=[
                                        SingleChoiceElement(name="yes", title=Title("yes")),
                                        SingleChoiceElement(name="no", title=Title("no")),
                                    ],
                                    prefill=DefaultValue("no"),
                                ),
                            ),
                        },
                    ),
                ),
                "perfdata_creation": DictElement(
                    required=False,
                    parameter_form=Dictionary(
                        title=Title("Perfdata creation"),
                        help_text=Help(
                            "By default, no performance data are generated. Define patterns here to select suites, tests and keywords which should be displayed in graphs. "
                            "Patterns always start at the beginning."
                        ),
                        elements={
                            "perfdata_creation_suites": DictElement(
                                required=False,
                                parameter_form=List(
                                    element_template=String(),
                                    title=Title("Suite perfdata"),
                                ),
                            ),
                            "perfdata_creation_tests": DictElement(
                                required=False,
                                parameter_form=List(
                                    element_template=String(),
                                    title=Title("Test perfdata"),
                                ),
                            ),
                            "perfdata_creation_keywords": DictElement(
                                required=False,
                                parameter_form=List(
                                    element_template=String(),
                                    title=Title("Keyword perfdata"),
                                ),
                            ),
                        },
                    ),
                ),
                "show_submessages": DictElement(
                    required=False,
                    parameter_form=BooleanChoice(
                        title=Title("Propagate the messages of child to parent nodes"),
                        help_text=Help(
                            "By default, suites and tests do not show messages of sub-items to save space. "
                            "Depending on the suite it can make sense to activate this setting to get a more descriptive output line."
                        ),
                        prefill=DefaultValue(False),
                    ),
                ),
                "show_kwmessages": DictElement(
                    required=False,
                    parameter_form=BooleanChoice(
                        title=Title("Show messages of keywords"),
                        help_text=Help(
                            "The 'messages' of keywords can give an insight of what the keyword has done; but depending on the "
                            "Robot libraries in use, this can make the output rather confusing."
                        ),
                        prefill=DefaultValue(False),
                    ),
                ),
                "includedate": DictElement(
                    required=False,
                    parameter_form=BooleanChoice(
                        title=Title("Include execution time in test/suites output line"),
                        help_text=Help(
                            "If checked, top level suites and tests will show their last execution."
                        ),
                        prefill=DefaultValue(False),
                    ),
                ),
            },
        )

    rule_spec_robotmk_check = CheckParameters(
        name="robotmk",
        topic=Topic.APPLICATIONS,
        parameter_form=_parameter_form_robotmk,
        title=Title("Robotmk v1 Monitoring"),
        condition=HostAndItemCondition(item_title=Title("Services")),
    )

# =============================================================================
# Legacy API Implementation (Checkmk 2.4 and below)
# =============================================================================
elif LEGACY_API_AVAILABLE:
    listof_runtime_threshold_suites = ListOf(
        Tuple(
            title=_("<b>Suite</b> thresholds"),
            show_titles=True,
            orientation="horizontal",
            elements=[
                TextAscii(
                    title=("<b>Suite</b> pattern"),
                    allow_empty=False,
                    size=60,
                ),
                Float(
                    title=("WARN threshold (sec)"),
                    size=19,
                ),
                Float(
                    title=("CRIT threshold (sec)"),
                    size=19,
                ),
            ],
        ),
        add_label=_("Add"),
        movable=False,
        title=_("<b>Suite</b> thresholds"),
    )

    listof_runtime_threshold_tests = ListOf(
        Tuple(
            title=_("<b>Test</b> thresholds"),
            show_titles=True,
            orientation="horizontal",
            elements=[
                TextAscii(
                    title=("<b>Test</b> pattern"),
                    allow_empty=False,
                    size=60,
                ),
                Float(
                    title=("WARN threshold (sec)"),
                    size=19,
                ),
                Float(
                    title=("CRIT threshold (sec)"),
                    size=19,
                ),
            ],
        ),
        add_label=_("Add"),
        movable=False,
        title=_("<b>Test</b> thresholds"),
    )

    listof_runtime_threshold_keywords = ListOf(
        Tuple(
            title=_("<b>Keyword</b> thresholds"),
            show_titles=True,
            orientation="horizontal",
            elements=[
                TextAscii(
                    title=("<b>Keyword</b> pattern"),
                    allow_empty=False,
                    size=60,
                ),
                Float(
                    title=("WARN threshold (sec)"),
                    size=19,
                ),
                Float(
                    title=("CRIT threshold (sec)"),
                    size=19,
                ),
            ],
        ),
        add_label=_("Add"),
        movable=False,
        title=_("<b>Keyword</b> thresholds"),
    )

    dropdown_robotmk_show_submessages = CascadingDropdown(
        title=_("Propagate the messages of child to parent nodes"),
        help=_(
            "By default, suites and tests do not show messages of sub-items to save space. <br>Depending on the suite it can make sense to activate this setting to get a more descriptive output line."
        ),
        choices=[
            (True, _("yes")),
            (False, _("no")),
        ],
        default_value=False,
    )
    dropdown_robotmk_show_kwmessages = CascadingDropdown(
        title=_("Show messages of keywords"),
        help=_(
            """
        The 'messages' of keywords can give an insight of what the keyword has done; but depending on the Robot libraries in use, this can make the output rather confusing. <br>
        Only set this to 'yes', if you rate this additional information as useful for the staff. 
        """
        ),
        choices=[
            (True, _("yes")),
            (False, _("no")),
        ],
        default_value=False,
    )

    dropdown_robotmk_show_all_runtimes = CascadingDropdown(
        title=_("Show monitored runtimes also when in OK state"),
        help=_(
            "By default, Robotmk only displays the runtime of Robot suites/tests/keywords where a threshold was exceeded. This helps to keep the output much cleaner. <br> "
            "To baseline newly created Robot tests for a certain time, it can be helpful to show even OK runtime values."
        ),
        choices=[
            ("yes", _("yes")),
            ("no", _("no")),
        ],
        default_value="no",
    )


    def _parameter_valuespec_robotmk():
        return Dictionary(
            elements=[
                (
                    "output_depth",
                    Dictionary(  # L1
                        title=_("Output depth"),
                        help=_(
                            "In Robot, suites and keywords can be nested. The default of Robotmk is to dissolve/recurse all nested objects and to show them in the service output.<br> "
                            "This is good in general, but sometimes not what you want (think of a keyword which is defined by five layers of abstraction).<br>"
                            "To keep the Robotmk output clear and understandable, set a proper pattern and e.g. <i>output depth=0</i> for sub-suites or keywords which should not get dissolved any deeper.<br>"
                            "(Hint: This is only for visual control; suites/keywords which are hidden by this setting can still be compared to <i>runtime_threshold</i> and change the overall suite state.)<br>"
                            "Patterns always start at the beginning."
                        ),
                        elements=[
                            (
                                "output_depth_suites",
                                ListOf(  # /L2
                                    Tuple(  # L3
                                        title=_("<b>Suite</b> Output depth"),
                                        show_titles=True,
                                        orientation="horizontal",
                                        elements=[
                                            TextAscii(
                                                title=("<b>Suite</b> pattern"),
                                                allow_empty=False,
                                                size=60,
                                            ),
                                            Integer(
                                                title=("depth"),
                                                size=3,
                                            ),
                                        ],
                                    ),  # L3 / Tuple
                                    add_label=_("Add"),
                                    movable=False,
                                    title=_("<b>Suite</b> Output depth"),
                                ),
                            ),  # L2 / output_depth_suites
                            (
                                "output_depth_keywords",
                                ListOf(  # /L2
                                    Tuple(  # L3
                                        title=_("<b>Keyword</b> Output depth"),
                                        show_titles=True,
                                        orientation="horizontal",
                                        elements=[
                                            TextAscii(
                                                title=("<b>Keyword</b> pattern"),
                                                allow_empty=False,
                                                size=60,
                                            ),
                                            Integer(
                                                title=("depth"),
                                                size=3,
                                            ),
                                        ],
                                    ),  # L3 / Tuple
                                    add_label=_("Add"),
                                    movable=False,
                                    title=_("<b>Keyword</b> Output depth"),
                                ),
                            ),  # L2 / output_depth_suites
                        ],
                    ),
                ),  # L1 / output_depth
                (
                    "runtime_threshold",
                    Dictionary(
                        title=_("Runtime thresholds"),
                        help=_(
                            "Define patterns here to assign runtime thresholds to suites, tests and keywords. <br>"
                            "A runtime exceedance always results in a WARN state and is propagated to the overall suite status.<br>"
                            "Always keep in mind that runtime monitoring is not a feature of Robot Framework but Robotmk; a Robot suite can have an internal OK state but be WARN in Checkmk!<br>"
                            "Patterns always start at the beginning. CRIT threshold must be bigger than WARN; values of 0 disable the threshold."
                        ),
                        elements=[
                            ("runtime_threshold_suites", listof_runtime_threshold_suites),
                            ("runtime_threshold_tests", listof_runtime_threshold_tests),
                            (
                                "runtime_threshold_keywords",
                                listof_runtime_threshold_keywords,
                            ),
                            ("show_all_runtimes", dropdown_robotmk_show_all_runtimes),
                        ],
                    ),
                ),  # L1 / runtime_threshold
                (
                    "perfdata_creation",
                    Dictionary(
                        title=_("Perfdata creation"),
                        help=_(
                            "By default, no performance data are generated. Define patterns here to select suites, tests and keywords which should be displayed in graphs. <br>"
                            "Patterns always start at the beginning."
                        ),
                        elements=[
                            (
                                "perfdata_creation_suites",
                                ListOfStrings(  # /L2
                                    title=_("<b>Suite</b> perfdata"),
                                    orientation="horizontal",
                                    size=60,
                                ),
                            ),  # L2
                            (
                                "perfdata_creation_tests",
                                ListOfStrings(  # /L2
                                    title=_("<b>Test</b> perfdata"),
                                    orientation="horizontal",
                                    size=60,
                                ),
                            ),  # L2
                            (
                                "perfdata_creation_keywords",
                                ListOfStrings(  # /L2
                                    title=_("<b>Keyword</b> perfdata"),
                                    orientation="horizontal",
                                    size=60,
                                ),
                            ),  # L2
                        ],
                    ),
                ),  # L1 / perfdata_creation
                (
                    "includedate",
                    DropdownChoice(
                        title=_("Include execution time in test/suites output line"),
                        help=_(
                            "If checked, top level suites and tests will show their last execution."
                        ),
                        choices=[
                            ("yes", _("yes")),
                            ("no", _("no")),
                        ],
                        default_value="no",
                    ),
                ),
                ("show_submessages", dropdown_robotmk_show_submessages),
                ("show_kwmessages", dropdown_robotmk_show_kwmessages),
            ],
        )


    def _item_spec_robotmk():
        return TextAscii(
            title=_("Services"),
            help=_(
                "Matches the service names generated from <u>Robot suites</u>. By default this is always the <i>topmost</i> suite (level 0) which results in <i>one service</i>.<br> "
                "Robot suites can be nested; to define a lower level CMK should "
                "generate services from, use the service discovery rule "
                "<i>Robotmk v1 Service Discovery</i>.<br>"
            ),
        )


    rulespec_registry.register(
        CheckParameterRulespecWithItem(
            check_group_name="robotmk",
            # gui/plugins/wato/utils/__init__.py
            group=RulespecGroupCheckParametersApplications,
            item_spec=_item_spec_robotmk,
            parameter_valuespec=_parameter_valuespec_robotmk,
            title=lambda: _("Robotmk v1 Monitoring"),
        )
    )

# Test imports when run standalone
if __name__ == "__main__":
    print(f"USES_V1_API: {USES_V1_API}")
    if USES_V1_API:
        print("✓ Successfully imported v1 API")
        print(f"✓ Rule spec created: {rule_spec_robotmk_check.name}")
    elif LEGACY_API_AVAILABLE:
        print("✓ Successfully imported legacy API")
        print("✓ Rule spec will be registered in GUI context")
    else:
        print("✗ No API available (likely standalone execution without GUI)")
    print("File loads successfully!")
