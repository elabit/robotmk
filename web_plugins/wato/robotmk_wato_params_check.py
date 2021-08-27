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
                               Tuple, Float)

from cmk.gui.plugins.wato import (
    CheckParameterRulespecWithItem,
    rulespec_registry,
    RulespecGroupCheckParametersApplications,
)

listof_runtime_threshold_suites = ListOf(Tuple(
    title=_('<b>Suite</b> thresholds'),
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
                                         title=_("<b>Suite</b> thresholds"))

listof_runtime_threshold_tests = ListOf(Tuple(
    title=_('<b>Test</b> thresholds'),
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
                                        title=_("<b>Test</b> thresholds"))

listof_runtime_threshold_keywords = ListOf(
    Tuple(
        title=_('<b>Keyword</b> thresholds'),
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
    title=_("<b>Keyword</b> thresholds"))

dropdown_robotmk_show_submessages = CascadingDropdown(
    title=_("Propagate the messages of child to parent nodes"),
    help=
    _("By default, suites and tests do not show messages of sub-items to save space. <br>Depending on the suite it can make sense to activate this setting to get a more descriptive output line."
      ),
    choices=[
        (True, _('yes')),
        (False, _('no')),
    ],
    default_value=False,
)
dropdown_robotmk_show_kwmessages = CascadingDropdown(
    title=_("Show messages of keywords"),
    help=_("""
    The 'messages' of keywords can give an insight of what the keyword has done; but depending on the Robot libraries in use, this can make the output rather confusing. <br>
    Only set this to 'yes', if you rate this additional information as useful for the staff. 
    """),
    choices=[
        (True, _('yes')),
        (False, _('no')),
    ],
    default_value=False,
)

dropdown_robotmk_show_all_runtimes = CascadingDropdown(
    title=_("Show monitored runtimes also when in OK state"),
    help=
    _("By default, Robotmk only displays the runtime of Robot suites/tests/keywords where a threshold was exceeded. This helps to keep the output much cleaner. <br> "
      "To baseline newly created Robot tests for a certain time, it can be helpful to show even OK runtime values."
      ),
    choices=[
        ('yes', _('yes')),
        ('no', _('no')),
    ],
    default_value="no",
)


def _parameter_valuespec_robotmk():
    return Dictionary(
        elements=[
            (
                "output_depth",
                Dictionary(  # L1
                    title=_('Output depth'),
                    help=
                    _('In Robot, suites and keywords can be nested. The default of Robotmk is to dissolve/recurse all nested objects and to show them in the service output.<br> '
                      'This is good in general, but sometimes not what you want (think of a keyword which is defined by five layers of abstraction).<br>'
                      'To keep the Robotmk output clear and understandable, set a proper pattern and e.g. <i>output depth=0</i> for sub-suites or keywords which should not get dissolved any deeper.<br>'
                      '(Hint: This is only for visual control; suites/keywords which are hidden by this setting can still be compared to <i>runtime_threshold</i> and change the overall suite state.)<br>'
                      'Patterns always start at the beginning.'),
                    elements=[
                        (
                            "output_depth_suites",
                            ListOf(  # /L2
                                Tuple(  # L3
                                    title=_('<b>Suite</b> Output depth'),
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
                                title=_("<b>Suite</b> Output depth"))
                        ),  # L2 / output_depth_suites
                        (
                            "output_depth_keywords",
                            ListOf(  # /L2
                                Tuple(  # L3
                                    title=_('<b>Keyword</b> Output depth'),
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
                                title=_("<b>Keyword</b> Output depth"))
                        ),  # L2 / output_depth_suites
                    ],
                )),  # L1 / output_depth
            ("runtime_threshold",
             Dictionary(
                 title=_('Runtime thresholds'),
                 help=
                 _('Define patterns here to assign runtime thresholds to suites, tests and keywords. <br>'
                   'A runtime exceedance always results in a WARN state and is propagated to the overall suite status.<br>'
                   'Always keep in mind that runtime monitoring is not a feature of Robot Framework but Robotmk; a Robot suite can have an internal OK state but be WARN in Checkmk!<br>'
                   'Patterns always start at the beginning. CRIT threshold must be bigger than WARN; values of 0 disable the threshold.'
                   ),
                 elements=[
                     ("runtime_threshold_suites",
                      listof_runtime_threshold_suites),
                     ("runtime_threshold_tests",
                      listof_runtime_threshold_tests),
                     ("runtime_threshold_keywords",
                      listof_runtime_threshold_keywords),
                     ("show_all_runtimes", dropdown_robotmk_show_all_runtimes),
                 ],
             )),  # L1 / runtime_threshold
            (
                "perfdata_creation",
                Dictionary(
                    title=_('Perfdata creation'),
                    help=_(
                        'By default, no performance data are generated. Define patterns here to select suites, tests and keywords which should be displayed in graphs. <br>'
                        'Patterns always start at the beginning.'),
                    elements=[
                        (
                            "perfdata_creation_suites",
                            ListOfStrings(  # /L2
                                title=_('<b>Suite</b> perfdata'),
                                orientation="horizontal",
                                size=60,
                            )),  # L2
                        (
                            "perfdata_creation_tests",
                            ListOfStrings(  # /L2
                                title=_('<b>Test</b> perfdata'),
                                orientation="horizontal",
                                size=60,
                            )),  # L2
                        (
                            "perfdata_creation_keywords",
                            ListOfStrings(  # /L2
                                title=_('<b>Keyword</b> perfdata'),
                                orientation="horizontal",
                                size=60,
                            )),  # L2
                    ],
                )),  # L1 / perfdata_creation
            ("includedate",
             DropdownChoice(
                 title=_("Include execution time in test/suites output line"),
                 help=
                 _("If checked, top level suites and tests will show their last execution."
                   ),
                 choices=[
                     ('yes', _('yes')),
                     ('no', _('no')),
                 ],
                 default_value="no",
             )),
            ("show_submessages", dropdown_robotmk_show_submessages),
            ("show_kwmessages", dropdown_robotmk_show_kwmessages),
        ], )


def _item_spec_robotmk():
    return TextAscii(
        title=_("Services"),
        help=_(
            "Matches the service names generated from <u>Robot suites</u>. By default this is always the <i>topmost</i> suite (level 0) which results in <i>one service</i>.<br> "
            "Robot suites can be nested; to define a lower level CMK should "
            "generate services from, use the service discovery rule "
            "<i>Robot Framework Service Discovery</i>.<br>"))


rulespec_registry.register(
    CheckParameterRulespecWithItem(
        check_group_name="robotmk",
        # gui/plugins/wato/utils/__init__.py
        group=RulespecGroupCheckParametersApplications,
        item_spec=_item_spec_robotmk,
        parameter_valuespec=_parameter_valuespec_robotmk,
        title=lambda: _("Robot Framework"),
    ))
