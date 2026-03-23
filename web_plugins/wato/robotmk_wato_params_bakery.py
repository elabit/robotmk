#!/usr/bin/python

# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

# ============================================================================
# Checkmk Version Detection and API Compatibility Layer
# ============================================================================

# Detect Checkmk version
try:
    from cmk.utils.paths import omd_root
    from cmk.ccc import version
    cmk_version = version.get_general_version_infos(omd_root)['version']
except ImportError:
    # Checkmk 2.2-2.4
    from cmk.utils.version import get_general_version_infos
    cmk_version = get_general_version_infos()['version']

IS_CMK_25_OR_LATER = cmk_version.startswith('2.5') or cmk_version.startswith('2.6')

if IS_CMK_25_OR_LATER:
    # ========================================================================
    # Checkmk 2.5+ Compatibility Layer
    # ========================================================================
    # Import new API
    from cmk.rulesets.v1 import Title as _Title, Help as _Help, Label as _Label
    from cmk.rulesets.v1.form_specs import (
        DictElement,
        Dictionary as _Dictionary25,
        String as _String25,
        Integer as _Integer25,
        SingleChoice as _SingleChoice25,
        CascadingSingleChoice as _CascadingSingleChoice25,
        List as _List25,
        FixedValue as _FixedValue25,
        DefaultValue,
        InputHint,
        SingleChoiceElement,
        CascadingSingleChoiceElement,
    )
    from cmk.rulesets.v1.rule_specs import AgentConfig, Topic
    from cmk.gui.log import logger
    # Import Tuple from unstable API for backward compatibility
    try:
        from cmk.gui.form_specs.unstable.legacy_converter import Tuple as _Tuple25
    except ImportError:
        # If unstable API not available, create a fallback
        _Tuple25 = None
    
    # Translation wrapper
    class _TextWrapper:
        """Makes translated strings compatible with Title/Help/Label"""
        def __init__(self, text):
            self.text = str(text) if text is not None else ""
        def __str__(self):
            return self.text
        def localize(self, *args, **kwargs):
            return self.text
    
    def _(text):
        """Translation function compatible with both APIs"""
        return _TextWrapper(text)
    
    # Adapter classes: Old API → New API
    class Dictionary:
        """Adapter: Dictionary (old API) → Dictionary (new API)"""
        def __init__(self, elements=None, title=None, help=None, optional_keys=None, **kwargs):
            self.elements = elements or []
            self.title = title
            self.help = help
            # Ensure optional_keys is always a list, never None or False
            if optional_keys is None or optional_keys is False:
                self.optional_keys = []
            elif not isinstance(optional_keys, (list, tuple)):
                self.optional_keys = list(optional_keys)
            else:
                self.optional_keys = optional_keys
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            elements_dict = {}
            for key, valuespec in self.elements:
                form_spec = valuespec._convert_to_25() if hasattr(valuespec, '_convert_to_25') else valuespec
                required = key not in self.optional_keys
                elements_dict[key] = DictElement(parameter_form=form_spec, required=required)
            
            return _Dictionary25(
                title=_Title(str(self.title)) if self.title else None,
                help_text=_Help(str(self.help)) if self.help else None,
                elements=elements_dict,
            )
    
    class ListOf:
        """Adapter: ListOf (old API) → List (new API)"""
        def __init__(self, valuespec, title=None, help=None, add_label=None, movable=True, **kwargs):
            self.valuespec = valuespec
            self.title = title
            self.help = help
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            element_template = self.valuespec._convert_to_25() if hasattr(self.valuespec, '_convert_to_25') else self.valuespec
            return _List25(
                title=_Title(str(self.title)) if self.title else None,
                help_text=_Help(str(self.help)) if self.help else None,
                element_template=element_template,
            )
    
    class ListOfStrings(ListOf):
        """Adapter: ListOfStrings (old API) → List of Strings (new API)"""
        def __init__(self, title=None, help=None, **kwargs):
            # ListOfStrings in old API was ListOf with TextAscii as default valuespec
            valuespec = TextAscii()
            super().__init__(valuespec=valuespec, title=title, help=help, **kwargs)
    
    class TextAscii:
        """Adapter: TextAscii (old API) → String (new API)"""
        def __init__(self, title=None, help=None, allow_empty=True, size=None, default_value=None, **kwargs):
            self.title = title
            self.help = help
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            return _String25(
                title=_Title(str(self.title)) if self.title else None,
                help_text=_Help(str(self.help)) if self.help else None,
            )
    
    # Aliases
    TextUnicode = TextAscii
    MonitoredHostname = TextAscii
    
    class Integer:
        """Adapter: Integer (old API) → Integer (new API)"""
        def __init__(self, title=None, help=None, minvalue=None, maxvalue=None, default_value=None, **kwargs):
            self.title = title
            self.help = help
            self.default_value = default_value
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            return _Integer25(
                title=_Title(str(self.title)) if self.title else None,
                help_text=_Help(str(self.help)) if self.help else None,
                prefill=DefaultValue(self.default_value) if self.default_value is not None else InputHint(0),
            )
    
    class Age:
        """Adapter: Age (old API) → Integer (new API) - represents seconds"""
        def __init__(self, title=None, help=None, minvalue=None, maxvalue=None, default_value=None, **kwargs):
            self.title = title
            self.help = help
            self.default_value = default_value
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            return _Integer25(
                title=_Title(str(self.title)) if self.title else None,
                help_text=_Help(str(self.help)) if self.help else None,
                prefill=DefaultValue(self.default_value) if self.default_value is not None else InputHint(0),
            )
    
    def _safe_identifier(value):
        """Convert a value to a safe Python identifier"""
        if isinstance(value, bool):
            # Use lowercase for booleans to avoid Python reserved keywords
            return "true" if value else "false"
        elif value is None:
            return "none"
        else:
            # Convert to string and ensure it's a valid identifier
            str_val = str(value)
            # Replace invalid characters
            if str_val and (str_val[0].isdigit() or not str_val.replace('_', '').isalnum()):
                return f"value_{str_val.replace(' ', '_').replace('-', '_')}"
            return str_val
    
    class DropdownChoice:
        """Adapter: DropdownChoice (old API) → SingleChoice (new API)"""
        def __init__(self, title=None, help=None, choices=None, default_value=None, sorted=True, **kwargs):
            self.title = title
            self.help = help
            self.choices = choices or []
            self.default_value = default_value
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            # Convert all choice values to safe identifiers for SingleChoiceElement names
            # Note: Boolean values become "true"/"false" strings in CMK 2.5
            elements = []
            for c in self.choices:
                choice_val = c[0]
                choice_title = c[1]
                element_name = _safe_identifier(choice_val)
                elements.append(SingleChoiceElement(name=element_name, title=_Title(str(choice_title))))
            
            # Default value handling
            default_name = _safe_identifier(self.default_value) if self.default_value is not None else (elements[0].name if elements else "")
            
            return _SingleChoice25(
                title=_Title(str(self.title)) if self.title else None,
                help_text=_Help(str(self.help)) if self.help else None,
                elements=elements,
                prefill=DefaultValue(default_name),
            )
    
    class CascadingDropdown:
        """Adapter: CascadingDropdown (old API) → CascadingSingleChoice (new API)
        Always returns tuples in CMK 2.5; bakery script handles tuple extraction"""
        def __init__(self, title=None, help=None, choices=None, default_value=None, sorted=True, **kwargs):
            self.title = title
            self.help = help
            self.choices = choices or []
            self.default_value = default_value
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            # Always use CascadingSingleChoice, even for parameter-less choices
            # The bakery script will extract values from tuples
            elements = []
            for choice in self.choices:
                choice_id, choice_title = choice[0], choice[1]
                choice_valuespec = choice[2] if len(choice) > 2 else None
                
                if choice_valuespec:
                    parameter_form = choice_valuespec._convert_to_25() if hasattr(choice_valuespec, '_convert_to_25') else choice_valuespec
                else:
                    # Use FixedValue(None) for choices without parameters
                    parameter_form = _FixedValue25(value=None)
                
                elements.append(CascadingSingleChoiceElement(
                    name=_safe_identifier(choice_id),
                    title=_Title(str(choice_title)),
                    parameter_form=parameter_form
                ))
            
            # For prefill, use the safe identifier of the default value, or first element
            default_name = _safe_identifier(self.default_value) if self.default_value is not None else (elements[0].name if elements else "")
            return _CascadingSingleChoice25(
                title=_Title(str(self.title)) if self.title else None,
                help_text=_Help(str(self.help)) if self.help else None,
                elements=elements,
                prefill=DefaultValue(default_name),
            )
    
    class Tuple:
        """Adapter: Tuple (old API) → Tuple/List (new API)"""
        def __init__(self, elements=None, title=None, help=None, orientation="vertical", **kwargs):
            self.elements = elements or []
            self.title = title
            self.help = help
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            converted = [e._convert_to_25() if hasattr(e, '_convert_to_25') else e for e in self.elements]
            if _Tuple25 is not None:
                # Use unstable Tuple if available
                return _Tuple25(
                    title=_Title(str(self.title)) if self.title else None,
                    help_text=_Help(str(self.help)) if self.help else None,
                    elements=converted,
                )
            else:
                # Fallback: Convert to List (less ideal but works)
                if len(converted) == 1:
                    # Single element tuple - just return the element
                    return converted[0]
                else:
                    # Multiple elements - wrap in List
                    # Note: This changes semantics but is better than failing
                    return _List25(
                        title=_Title(str(self.title)) if self.title else None,
                        help_text=_Help(str(self.help)) if self.help else None,
                        element_template=converted[0] if converted else _String25(),
                    )
    
    class Transform:
        """Adapter: Transform - wraps valuespec and preserves transform functions
        In CMK 2.5, transformations aren't directly supported, so we just return the converted form spec"""
        def __init__(self, valuespec, forth=None, back=None, **kwargs):
            self.valuespec = valuespec
            self.forth = forth
            self.back = back
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            # Convert the wrapped valuespec to form spec
            # Note: forth/back transformations aren't supported in CMK 2.5's form specs
            # The transformation will need to be handled elsewhere (e.g., in the bakery script)
            return self.valuespec._convert_to_25() if hasattr(self.valuespec, '_convert_to_25') else self.valuespec
    
    class Alternative:
        """Adapter: Alternative (old API) → CascadingSingleChoice (new API)"""
        def __init__(self, title=None, help=None, elements=None, style="dropdown", **kwargs):
            self.title = title
            self.help = help
            self.elements = elements or []
            self.style = style
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            choice_elements = []
            for idx, elem in enumerate(self.elements):
                param_form = elem._convert_to_25() if hasattr(elem, '_convert_to_25') else elem
                elem_title = getattr(elem, 'title', f"Option {idx + 1}")
                choice_elements.append(CascadingSingleChoiceElement(
                    name=f"option_{idx}",
                    title=_Title(str(elem_title)),
                    parameter_form=param_form
                ))
            
            return _CascadingSingleChoice25(
                title=_Title(str(self.title)) if self.title else None,
                help_text=_Help(str(self.help)) if self.help else None,
                elements=choice_elements,
                prefill=DefaultValue("option_0") if choice_elements else DefaultValue(""),
            )
    
    class FixedValue:
        """Adapter: FixedValue (old API) → FixedValue (new API)"""
        def __init__(self, value, title=None, totext=None, **kwargs):
            self.value = value
            self.title = title
            self.totext = totext
            self.kwargs = kwargs
        
        def _convert_to_25(self):
            return _FixedValue25(
                title=_Title(str(self.title)) if self.title else None,
                help_text=_Help(str(self.totext)) if self.totext else None,
                value=self.value,
            )
    
    # Placeholder for old API registration - will be replaced at module end
    class _MockRulespecRegistry:
        def register(self, rulespec):
            pass
    
    rulespec_registry = _MockRulespecRegistry()
    
    class HostRulespec:
        """Placeholder for old HostRulespec"""
        def __init__(self, **kwargs):
            self.kwargs = kwargs
    
    class RulespecGroupMonitoringAgentsAgentPlugins:
        """Placeholder for old group"""
        pass

else:
    # ========================================================================
    # Checkmk < 2.5: Use old API directly
    # ========================================================================
    from cmk.gui.i18n import _
    from cmk.gui.valuespec import (
        DropdownChoice,
        Dictionary,
        ListOf,
        TextAscii,
        TextUnicode,
        Tuple,
        CascadingDropdown,
        Integer,
        Transform,
        Alternative,
        FixedValue,
        Age,
        MonitoredHostname,
        ListOfStrings,
    )
    from cmk.gui.plugins.wato import (
        rulespec_registry,
        HostRulespec,
    )
    from cmk.gui.log import logger
    from cmk.gui.cee.plugins.wato.agent_bakery.rulespecs.utils import (
        RulespecGroupMonitoringAgentsAgentPlugins,
    )

# ============================================================================
# Original Robotmk Configuration Code (Works with both APIs via adapters)
# ============================================================================

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
class RMKConfig:
    def __init__(self):
        self._cfg_dict = {
            "global": DictNoNone(),
            "suites": DictNoNone(),
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
        return data

    @classmethod
    def wato_forth(cls, data):
        """Convert the canonical data structure coming from the rule to present in WATO"""
        return data

    @classmethod
    def from_env(cls):
        """Creates the RMK Config from environment variables (TBD)"""
        rmk_config = RMKConfig()
        return rmk_config._conf

    @property
    def global_dict(self):
        return self._cfg_dict["global"]

    @property
    def suites_dict(self):
        return self.cfg_dict["suites"]

    # ------------------------
    @property
    def execution_mode(self):
        return self.global_dict["execution_mode"]

    @execution_mode.setter
    def execution_mode(self, val):
        self.global_dict["execution_mode"] = val

    # ------------------------
    @property
    def agent_output_encoding(self):
        return self.global_dict["agent_output_encoding"]

    @agent_output_encoding.setter
    def agent_output_encoding(self, val):
        self.global_dict["agent_output_encoding"] = val

    # ------------------------
    @property
    def transmit_html(self):
        return self.global_dict["transmit_html"]

    @transmit_html.setter
    def transmit_html(self, val):
        self.global_dict["transmit_html"] = val

    # ------------------------
    @property
    def logging(self):
        return self.global_dict["logging"]

    @logging.setter
    def logging(self, val):
        self.global_dict["logging"] = val

    # ------------------------
    @property
    def log_rotation(self):
        return self.global_dict["log_rotation"]

    @log_rotation.setter
    def log_rotation(self, val):
        self.global_dict["log_rotation"] = val

    # ------------------------
    @property
    def robotdir(self):
        return self.global_dict["robotdir"]

    @robotdir.setter
    def robotdir(self, val):
        self.global_dict["robotdir"] = val

    # ------------------------
    @property
    def outputdir(self):
        return self.global_dict["outputdir"]

    @outputdir.setter
    def outputdir(self, val):
        self.global_dict["outputdir"] = val


# EXECUTION MODE Help Texts --------------------------------
_helptext_execution_mode_agent_serial = """
    The Checkmk agent starts the Robotmk <b>controller</b> as a <i>synchronous</i> check plugin in the <i>agent check interval</i>.<br>
    Simultanously, the agent starts the Robotmk <b>runner</b> as an <i>asynchronous</i> check plugin in the <i>runner execution interval</i>.<br>
    If you do not specify suites, the runner will execute all suites in the <i>Robot suites directory</i>. <br><br>
    <b>Use cases</b> for this mode:<br>
    In general, all Robot tests which can run headless and do not require a certain OS user."""
_helptext_execution_mode_agent_parallel = """(not yet implemented)"""
_helptext_execution_mode_external = """
    The Checkmk agent starts the Robotmk <b>controller</b> as a <i>synchronous</i> check plugin in the <i>agent check interval</i>.<br><br>
    <b>Important note for Checkmk 1.6</b>: The rule <i>Deploy custom files with agent</i> (package <tt>robotmk-external</tt>) must be used to place the <b>runner</b> within the agent's <tt>bin</tt> directory (there is no other way in Checkmk 1 to deploy files to that folder).<br><br>
    You can start the runner from the <tt>bin</tt> folder with any external tool (e.g. systemd timer/cron/task scheduler) and in the user context of your choice.<br>
    Make sure that the output/log dir (see below) can be written by the user which executes <tt>robotmk-runner.py</tt> or choose another location with the setting <i>Change default directories</i> below.<br><br>
    If no suites are specified, the runner will execute all suites listed in <tt>robotmk.yml</tt>.<br>
    If no suites are defined at all, the runner will execute all suites found in the <i>Robot suites directory</i>. <br><br>   
    <b>Use cases</b> for this mode: <br>
      - Desktop Applications<br>
      - Applications which require to be run with a certain user account (SSO)<br>
      - The need for more control about when to execute a Robot test and when not"""

# GLOBAL EXECUTION INTERVAL: only serial ===========================================================
_agent_config_global_suites_execution_interval_agent_serial = Age(
    title=_("Runner <b>execution interval</b>"),
    help=_(
        "This configures the interval in which the Checkmk agent will execute the <b>runner</b> plugin asynchronously.<br>"
        "The default is 15min but strongly depends on the maximum probable runtime of all <i>test suites</i>.<br>Choose an interval which is a good comprimise between frequency and execution runtime headroom.<br>"
    ),
    minvalue=1,
    maxvalue=65535,
    default_value=900,
)

# GLOBAL CACHE TIME: serial & external =============================================================
_agent_config_global_cache_time_agent_serial = Age(
    title=_("Result <b>cache time</b>"),
    help=_(
        "Suite state files are updated by the <b>runner</b> after each execution (<i>Runner execution interval</i>).<br>"
        "The <b>controller</b> monitors the age of those files and expects them to be not older than the <i>global cache time</i>. <br>"
        "Each suite with a state file older than its <i>result cache time</i> will be reported as 'stale'.<br>"
        "For obvious reasons, the cache time must always be set higher than the <i>runner execution interval</i>, including reruns of failed tests/subsuites (if configured).<br>"
        "(Do not confuse it with the <i>cache time</i> which Checkmk uses for the agent plugin configuration.)"
    ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)
_agent_config_global_cache_time_external = Age(
    title=_("Result <b>cache time</b>"),
    help=_(
        "Suite state files are updated every time when the <b>runner</b> has executed the suites.<br>"
        "The <b>controller</b> monitors the age of those files and expects them to be not older than the <i>global cache time</i> or the <i>suite cache time</i> (if set). <br>"
        "Each suite with a state file older than its <i>cache time</i> will be reported as 'stale'.<br>"
        "For obvious reasons, this cache time must always be set higher than the execution interval, including reruns of failed tests/subsuites (if configured)."
    ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

# SUITE CACHE TIMES: parallel & external ===========================================================
_agent_config_suite_suites_cache_time_agent_parallel = Age(
    title=_("Suite cache time"),
    help=_(
        "Sets the <b>suite specific</b> cache time. (Must be higher than the <i>suite execution interval</i>, including reruns of failed tests/subsuites)"
    ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

_agent_config_suite_suites_cache_time_external = Age(
    title=_("Suite cache time"),
    help=_(
        "Sets <b>suite specific cache times</b> for <b>individual execution intervals, including reruns of failed tests/subsuites</b>"
    ),
    minvalue=1,
    maxvalue=65535,
    default_value=960,
)

# SUITE EXECUTION INTERVAL: only parallel ==========================================================
_agent_config_suite_suites_execution_interval_agent_parallel = Age(
    title=_("Suite execution interval"),
    help=_(
        "Sets the interval in which the Robotmk <b>controller</b> will trigger the Robotmk <b>runner</b> to execute <b>this particular suite</b>.<br>"
    ),
    minvalue=1,
    maxvalue=65535,
    default_value=900,
)


_agent_config_testsuites_tag = TextUnicode(
    title=_("Unique suite tag"),
    help=_(
        "Suites which are <b>added multiple times</b> (to execute them with different parameters) should have a <b>unique tag</b>.<br>"
    ),
    allow_empty=False,
    size=30,
)


_agent_config_dict_dirs = Dictionary(
    title=_("Change <b>default directories</b>"),
    help=_("This settings allow to override paths where Robotmk stores files. "),
    elements=[
        (
            "robotdir",
            TextUnicode(
                help=_(
                    "Defines where the Robotmk plugin will search for <b>Robot suites</b>. By default this is:<br>"
                    " - <tt>/usr/lib/check_mk_agent/robot</tt> (Linux)<br>"
                    " - <tt>C:\\ProgramData\\checkmk\\agent\\robot</tt> (Windows) <br>"
                    "Windows paths can be given with single backslashes; OS dependent path validation is made during the agent baking.<br>"
                ),
                title=_("Robot suites directory (<tt>robotdir</tt>)"),
                allow_empty=False,
                size=100,
                default_value="",
            ),
        ),
        (
            "outputdir",
            TextUnicode(
                help=_(
                    "Defines where Robot Framework <b>XML/HTML</b> and the <b>Robotmk JSON state files</b> will be stored. By default this is:<br>"
                    " - <tt>/var/log/robotmk</tt> (Linux)<br>"
                    " - <tt>C:\\ProgramData\\checkmk\\agent\\log\\robotmk</tt> (Windows) <br>"
                    "Windows paths can be given with single backslashes; OS dependent path validation is made during the agent baking.<br>"
                ),
                title=_("Output directory"),
                allow_empty=False,
                size=100,
                default_value="",
            ),
        ),
        (
            "logdir",
            TextUnicode(
                help=_(
                    "Defines where Robotmk <b>controller/runner execution log files</b> will be written to. By default this is:<br>"
                    " - <tt>/var/log/robotmk</tt> (Linux)<br>"
                    " - <tt>C:\\ProgramData\\checkmk\\agent\\log\\robotmk</tt> (Windows) <br>"
                    "Windows paths can be given with single backslashes; OS dependent path validation is made during the agent baking.<br>"
                ),
                title=_("Log directory"),
                allow_empty=False,
                size=100,
                default_value="",
            ),
        ),
    ],
)

_agent_config_testsuites_piggybackhost = MonitoredHostname(
    title=_("Assign result to Piggyback host"),
    help=_(
        "Piggyback allows to assign the results of this particular Robot test to another host."
    ),
)

_agent_config_testsuites_path = TextUnicode(
    title=_("Robot test path"),
    help=_(
        "Name of the <tt>.robot</tt> file or directory containing <tt>.robot</tt> files, relative to the <i>robot suites directory</i><br>"
        "It is highly recommended to organize Robot suites in <i>directories</i> and to specify the directories here without leading/trailing (back)slashes.<br>"
        "💡 If a suite needs to be <b>skipped temporarily</b>, place a file <tt>DISABLED</tt> in the <i>robot suites directory</i>. Robotmk will silently omit the execution, detected services will be displayed as outdated/stale, but will not be alerted."
    ),
    allow_empty=False,
    size=50,
)

# TEST SELECTION DICT ELEMENTS =================================================
# To be used in test selection and rerunfailed
# Ref: 7uBbn2
_dict_el_suite_selection = (
    "suite",
    ListOfStrings(
        title=_("Select suites (<tt>--suite</tt>)"),
        help=_(
            "Select suites by name. <br>When this option is used with"
            " <tt>--test</tt>, <tt>--include</tt> or <tt>--exclude</tt>, only tests in"
            " matching suites and also matching other filtering"
            " criteria are selected. <br>"
            " Name can be a simple pattern similarly as with <tt>--test</tt> and it can contain parent"
            " name separated with a dot. <br>"
            " For example, <tt>X.Y</tt> selects suite <tt>Y</tt> only if its parent is <tt>X</tt>.<br>"
        ),
        size=40,
    ),
)
_dict_el_test_selection = (
    "test",
    ListOfStrings(
        title=_("Select test (<tt>--test</tt>)"),
        help=_(
            "Select tests by name or by long name containing also"
            " parent suite name like <tt>Parent.Test</tt>. <br>Name is case"
            " and space insensitive and it can also be a simple"
            " pattern where <tt>*</tt> matches anything, <tt>?</tt> matches any"
            " single character, and <tt>[chars]</tt> matches one character"
            " in brackets.<br>"
        ),
        size=40,
    ),
)
_dict_el_test_include = (
    "include",
    ListOfStrings(
        title=_("Include tests by tag (<tt>--include</tt>)"),
        help=_(
            'Select tests by tag. (<a href="https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#tagging-test-cases">About tagging test cases</a>)<br>Similarly as name with <tt>--test</tt>,'
            "tag is case and space insensitive and it is possible"
            "to use patterns with <tt>*</tt>, <tt>?</tt> and <tt>[]</tt> as wildcards.<br>"
            "Tags and patterns can also be combined together with"
            "<tt>AND</tt>, <tt>OR</tt>, and <tt>NOT</tt> operators.<br>"
            "Examples: <br><tt>foo</tt><br><tt>bar*</tt><br><tt>fooANDbar*</tt><br>"
        ),
        size=40,
    ),
)

_dict_el_test_exclude = (
    "exclude",
    ListOfStrings(
        title=_("Exclude tests by tag (<tt>--exclude</tt>)"),
        help=_(
            'Select test cases not to run at all by tag. (<a href="https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#tagging-test-cases">About tagging test cases</a>)<br>These tests are'
            " not run even if included with <tt>--include</tt>. <br>Tags are"
            " matched using same rules as with <tt>--include</tt>.<br>"
        ),
        size=40,
    ),
)


_dict_el_suite_argsfile = (
    "argumentfile",
    ListOfStrings(
        title=_("Load arguments from file (<tt>--argumentfile</tt>)"),
        help=_(
            "Name of files containing <b>additional command line arguments</b> for Robot Framework. The paths are relative to the <i>robot suites directory</i>.<br>"
            "Argument files allow placing all or some command line options and arguments into an external file where they will be read. This is useful for more exotic RF parameters not natively supported by Robotmk or for problematic characters.<br>"
            'The arguments given here are taken into use along with possible other command line options. (See also <a href="https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#argument-files">About argument files</a>)<br><br>'
        ),
        size=70,
    ),
)

_dict_el_suite_variablefile = (
    "variablefile",
    ListOfStrings(
        title=_("Load variables from file (<tt>--variablefile</tt>)"),
        help=_(
            'Python or YAML file file to read variables from. (<a href="https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#variable-files">About variable files</a>)<br>Possible arguments to the variable file can be given'
            " after the path using colon or semicolon as separator.<br>"
            "Examples:<br> "
            "<tt>path/vars.yaml</tt><br>"
            "<tt>set_environment.py:testing</tt><br>"
        ),
        size=70,
    ),
)

_agent_config_testsuites_robotframework_params_dict_base = Dictionary(
    title=_("Robot Framework parameters"),
    help=_(
        "The options here allow to specify the most common <b>commandline parameters</b> for Robot Framework.<br>"
        'In order to use other parameters (see <a href="https://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#all-command-line-options">All command line options</a>), you can use the option \'Load arguments from file\'.<br> (Feel free to <a href="https://github.com/elabit/robotmk/issues">file an issue</a> if you think that a special parameter should be added here)'
    ),
    elements=[
        (
            "name",
            TextUnicode(
                title=_("Top level suite name (<tt>--name</tt>)"),
                help=_(
                    "Set the name of the top level suite. By default the name is created based on the executed file or directory.<br>"
                    "This sets the name of a fresh discovered Robot service; an already existing service will hide away and will be found by the discovery under a new name."
                ),
                allow_empty=False,
                size=50,
            ),
        ),
        # Ref: 7uBbn2
        _dict_el_suite_selection,
        _dict_el_test_selection,
        _dict_el_test_include,
        _dict_el_test_exclude,
        (
            "variable",
            ListOf(
                Tuple(
                    elements=[
                        TextAscii(title=_("Variable name:")),
                        TextAscii(
                            title=_("Value:"),
                        ),
                    ],
                    orientation="horizontal",
                ),
                movable=False,
                title=_("Variables (<tt>--variable</tt>)"),
                help=_(
                    "Set variables in the test data. <br>Only scalar variables with string"
                    " value are supported and name is given without <tt>${}</tt>. <br>"
                    " (See <tt>--variablefile</tt> for a more powerful variable setting mechanism.)<br>"
                ),
            ),
        ),
        _dict_el_suite_variablefile,
        _dict_el_suite_argsfile,
        (
            "exitonfailure",
            DropdownChoice(
                title=_("Exit on failure (<tt>--exitonfailure</tt>)"),
                help=_(
                    """
                    By default, Robot Framework will execute <i>every</i> test.<br>
                    But sometimes tests are interdependent - in the event of a failed login, for example, it is impossible to still successfully complete the subsequent tests.<br>
                    If this option is active, Robot Framework will <b>immediately stop</b> the suite execution if a test fails. <br>
                    The results of subsequent tests (which would have failed) will then not be passed to Checkmk; depending on the discovery settings,
                    their results will either be <b>missing</b> (if within a suite result) or the services generated for them will <b>go stale</b>.<br> <br>  
                    <b>Important note</b>: this is where Robotmk deviates from Robot Framework behavior. The HTML log will still contain the omitted tests and show them as <tt>FAIL</tt> (even though they were not executed).<br>
                    See also "<a href=\"http://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#stopping-when-first-test-case-fails\">How to stop a suite when the first test fails</a>". 
                    """
                ),
                choices=[
                    ("yes", _("yes")),
                    ("no", _("no")),
                ],
                default_value="no",
            ),
        ),
    ],
)

# Wrap to filter empty defaults (empty lists, empty strings, empty dicts)
def _filter_empty_robot_params(data):
    """Remove empty values from robot_params to keep YAML clean"""
    if not isinstance(data, dict):
        return data
    filtered = {}
    for key, value in data.items():
        # Keep non-empty values and the exitonfailure field (even if 'no')
        if key == 'exitonfailure':
            filtered[key] = value
        elif isinstance(value, (list, dict, str)):
            if value:  # Only include if not empty
                filtered[key] = value
        else:
            filtered[key] = value
    return filtered

_agent_config_testsuites_robotframework_params_dict = Transform(
    valuespec=_agent_config_testsuites_robotframework_params_dict_base,
    forth=lambda x: x,  # No transformation needed when loading
    back=_filter_empty_robot_params,  # Filter empty values when saving
)

_agent_config_testsuites_max_executions_selection_dict = Dictionary(
    help=_(
        """
    With the following options it is possible to further filter the list of tests/suites to re-run. (Documentation: <a href=\"http://robotframework.org/robotframework/latest/RobotFrameworkUserGuide.html#re-executing-failed-test-cases\">Re-executing failed test cases</a>)
    """
    ),
    title=_("Filter"),
    elements=[
        # Ref: 7uBbn2
        _dict_el_suite_selection,
        _dict_el_test_selection,
        _dict_el_test_include,
        _dict_el_test_exclude,
    ],
)

_agent_config_testsuites_failed_handling_max_iterations = Integer(
   title=_("Maximum iteration attempts"),
    help=_("The maximum number of suite executions (including the first attempt)"),
    minvalue=1,
    default_value=2,
)


_agent_config_testsuites_failed_handling_dict = Dictionary(
    title=_("Handling of failed results"),
    help=_(
        """This section controls how often Robotmk repeats suites/tests after an <b>incorrect suite result</b>, even before it passes it on to the agent. <br>
            Use this feature only as a last resort, for example when applications behave unreliable. <br>
            (Also take into account that every re-execution requires additional headroom for the <i>result cache time</i>)."""
    ),
    optional_keys=False,
    elements=[
        ("max_iterations", _agent_config_testsuites_failed_handling_max_iterations),
        (
            "strategy",
            CascadingDropdown(
                title=_("Strategy"),
                choices=[
                    (
                        "incremental",
                        _("incremental"),
                        Dictionary(
                            help=_(
                                """Robotmk <b>executes only the failed tests</b> at each iteration, 
                                until either all tests are OK or the maximum allowed number of iterations is reached.<br>
                                In the end, the <b>last/best result per test</b> will be merged into the overall state.<br> 
                                Choose this mode if the tests within the robot suite do not depend on each other, but can be executed separately."""
                            ),
                            elements=[
                                (
                                    "rerun_selection",
                                    _agent_config_testsuites_max_executions_selection_dict,
                                ),
                            ],
                            optional_keys=False,
                        ),
                    ),
                    (
                        "complete",
                        _("complete"),
                        Dictionary(
                            help=_(
                                """Robotmk <b>re-runs always the entire suite</b> at each iteration - 
                                until either the suite result is OK or the maximum number of repetitions allowed is reached.<br>                                
                                Choose this mode if the tests within the robot suite are related to each other and their execution <br>
                                order is also crucial.<br> 
                                (Example: <tt>T1: Login, T2: Order 1st item, T3: Order 2nd item, T4: Check basket</tt>)"""
                            ),
                            optional_keys=False,
                            elements=[],
                        ),
                    ),
                ],
            ),
        ),
    ],
)


# Make the help text of SuitList dependent on the type of execution
def _gen_agent_config_dict_listof_testsuites(mode):
    titledict = {
        "agent_serial": "to execute",
        "agent_parallel": "to execute individually",
        "external": "to be executed externally",
    }
    return ListOf(
        _gen_testsuite_tuple(mode),
        help=_(
            """xClick on '<i>Add test suite</i>' to specify the suites to be executed, including additional parameters, 
            piggyback host and execution order. This is the recommended way.<br>
            If you do not add any suite here, the Robotmk plugin will add every <tt>.robot</tt> file/every directory 
            within the <i>Robot suites directory</i> to the execution list - without any further parametrization.<br>"""
        ),
        add_label=_("Add test suite"),
        movable=True,
        title=_("Suites"),
    )


def _agent_config_testsuites_failed_handling_forth(data):
    """This back/forth Transform changes the format in which the data are saved because CascadingDropdown produces a Tuple which cannot
    be written as YAML. It also helps to migrate from an older Robotmk version."""
    if not "strategy" in data:
        # Data coming from an older Robotmk version (do not contain the strategy key)
        max_iterations = data.get("max_executions", 2)
        rerun_selection = data.get("rerun_selection", {})
        new_data = {}
        new_data["max_iterations"] = max_iterations
        new_data["strategy"] = ("incremental", {"rerun_selection": rerun_selection})

    else:
        # Data coming from file
        # {'max_iterations': 2, 'strategy': {'name': 'incremental', 'rerun_selection': {'suite': ['asdas'], 'include': ['adasda']}}}
        if type(data["strategy"]) == dict:
            name = data["strategy"]["name"]
            if name == "incremental":
                new_strategy_tuple = (
                    name,
                    {"rerun_selection": data["strategy"]["rerun_selection"]},
                )
            else:
                new_strategy_tuple = (name, {})
        # {'max_iterations': 2, 'strategy': ('incremental', {'rerun_selection': {'test': ['sdfsd']}})}
        # Data format coming from vSpec, must be converted
        else:
            name = data["strategy"][0]
            if name == "incremental":
                new_strategy_tuple = (
                    name,
                    {"rerun_selection": data["strategy"][1]},
                )
            else:
                new_strategy_tuple = (name, {})

        new_data = {
            "max_iterations": data["max_iterations"],
            "strategy": new_strategy_tuple,
        }

    return new_data


def _agent_config_testsuites_failed_handling_back(data):
    """This back/forth Transform changes the format in which the data are saved because CascadingDropdown produces a Tuple which cannot
    be written as YAML. It also helps to migrate from an older Robotmk version."""
    #  'strategy': ('incremental', {'rerun_selection': {'test': ['sdfsd']}})}
    strategy = data["strategy"]
    max_iterations = data["max_iterations"]
    name = strategy[0]
    new_strategy = {
        "name": name,
    }
    if name == "incremental":
        rerun_selection = strategy[1]
        new_strategy.update(rerun_selection)
    new_data = {"max_iterations": max_iterations, "strategy": new_strategy}

    return new_data


_agent_config_testsuites_failed_handling_transform = Transform(
    _agent_config_testsuites_failed_handling_dict,
    forth=_agent_config_testsuites_failed_handling_forth,
    back=_agent_config_testsuites_failed_handling_back,
)


def _gen_testsuite_tuple(mode):
    if mode == "agent_serial":
        return Dictionary(
            elements=[
                ("path", _agent_config_testsuites_path),
                ("tag", _agent_config_testsuites_tag),
                ("piggybackhost", _agent_config_testsuites_piggybackhost),
                ("robot_params", _agent_config_testsuites_robotframework_params_dict),
                ("failed_handling", _agent_config_testsuites_failed_handling_transform),
            ],
            optional_keys=["tag", "piggybackhost", "robot_params", "failed_handling"],
        )

    if mode == "external":
        return Dictionary(
            elements=[
                ("path", _agent_config_testsuites_path),
                ("tag", _agent_config_testsuites_tag),
                ("piggybackhost", _agent_config_testsuites_piggybackhost),
                ("robot_params", _agent_config_testsuites_robotframework_params_dict),
                ("failed_handling", _agent_config_testsuites_failed_handling_transform),
            ],
            optional_keys=["tag", "piggybackhost", "robot_params", "failed_handling"],
        )


_dropdown_robotmk_output_encoding = CascadingDropdown(
    title=_("Agent output encoding"),
    help=_(
        """
        The agent payload of Robotmk is JSON with fields for <b>XML and HTML data</b> (which can contain embedded images). <br>
        To save bandwidth and resources, this fields are by default <b>zlib compressed</b> to 5% of their size.<br>
        Unless you are debugging or curious there should be no reason to change the encoding."""
    ),
    choices=[
        ("zlib_codec", _("Zlib compressed")),
        ("utf_8", _("UTF-8")),
        ("base64_codec", _("BASE-64")),
    ],
    default_value="zlib_codec",
)

_dropdown_robotmk_transmit_html = DropdownChoice(
    title=_("Transmit HTML log to Checkmk server"),
    help=_(
        """
    Robotmk transmits the <b>HTML log file</b> written by Robot Framework to the Checkmk server, where it can be action-linked with the discovered services. <br>
    This feature needs some <b>configuration</b> which you can find in the <b>Robotmk discovery rule</b>, option <i>'Restrict the HTML log files link creation'</i>.
    """
    ),
    choices=[
        (False, _("No")),
        (True, _("Yes")),
    ],
    default_value=False,
)

_dropdown_robotmk_log_rotation = CascadingDropdown(
    title=_("Number of days to keep Robot XML/HTML log files on the host"),
    help=_(
        "This setting helps to keep the test host clean by <b>deleting the log files</b> after a certain amount of days. Log files are: <br>"
        "<tt>robotframework-$SUITENAME-$timestamp-output.xml<br>"
        "<tt>robotframework-$SUITENAME-$timestamp-log.html<br>"
    ),
    choices=[
        (1, _("1")),
        (3, _("3")),
        (7, _("7")),
        (14, _("14")),
        (30, _("30")),
        (90, _("90")),
        (365, _("365")),
    ],
    default_value=7,
    sorted=False,
)

_dropdown_robotmk_logging = DropdownChoice(
    title=_("Robotmk log level"),
    help=_(
        """
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

_dropdown_robotmk_execution_choices = CascadingDropdown(
    title=_("Execution mode"),
    help=_(
        "The <b>execution mode</b> is a general setting which controls who runs RF suites, how and when.<br>"
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
        (
            "agent_serial",
            _("agent_serial"),
            Dictionary(
                help=_(_helptext_execution_mode_agent_serial),
                optional_keys=False,
                elements=[
                    (
                        "suites",
                        _gen_agent_config_dict_listof_testsuites("agent_serial"),
                    ),
                    ("cache_time", _agent_config_global_cache_time_agent_serial),
                    (
                        "execution_interval",
                        _agent_config_global_suites_execution_interval_agent_serial,
                    ),
                ],
            ),
        ),
        (
            "external",
            _("external"),
            Dictionary(
                help=_(_helptext_execution_mode_external),
                optional_keys=False,
                elements=[
                    ("suites", _gen_agent_config_dict_listof_testsuites("external")),
                    ("cache_time", _agent_config_global_cache_time_external),
                ],
            ),
        ),
    ],
)


def _valuespec_agent_config_robotmk():
    return Alternative(
        title=_("Robotmk v1 Agent Plugin (Linux, Windows)"),
        help=_(
            "Robotmk integrates the results of <b>Robot Framework</b> tests into Checkmk. This rule will deploy the <b>Robotmk agent plugin</b> and a generated YML control file (<tt>robotmk.yml</tt>) to the remote host."
        ),
        style="dropdown",
        elements=[
            Dictionary(
                title=_("Deploy the Robotmk plugin"),
                elements=[
                    ("execution_mode", _dropdown_robotmk_execution_choices),
                    ("agent_output_encoding", _dropdown_robotmk_output_encoding),
                    ("transmit_html", _dropdown_robotmk_transmit_html),
                    ("log_level", _dropdown_robotmk_logging),
                    ("log_rotation", _dropdown_robotmk_log_rotation),
                    ("dirs", _agent_config_dict_dirs),
                ],
                optional_keys=False,
            ),
            FixedValue(
                None,
                title=_("Do not deploy the Robotmk plugin"),
                totext=_("(No Robot Framework tests on this machine)"),
            ),
        ],
    )


# ============================================================================
# Registration - Version-specific
# ============================================================================

if IS_CMK_25_OR_LATER:
    # For Checkmk 2.5+, register using the new API
    def _parameter_form_robotmk():
        """Convert the old-style valuespec to new API"""
        # For CMK 2.5+, AgentConfig expects a Dictionary at the top level
        # The old Alternative allowed "deploy" vs "don't deploy", but in 2.5
        # the user simply doesn't create a rule if they don't want to deploy
        old_valuespec = _valuespec_agent_config_robotmk()
        
        # Get the Dictionary from the first option of the Alternative (Deploy option)
        first_option = old_valuespec.elements[0]
        return first_option._convert_to_25()
    
    rule_spec_robotmk_bakery = AgentConfig(
        name="robotmk",
        title=_Title("Robotmk v1 Agent Plugin (Linux, Windows)"),
        topic=Topic.GENERAL,
        parameter_form=_parameter_form_robotmk,
    )
else:
    # For Checkmk < 2.5, register using the old API
    rulespec_registry.register(
        HostRulespec(
            group=RulespecGroupMonitoringAgentsAgentPlugins,
            name="agent_config:robotmk",
            valuespec=_valuespec_agent_config_robotmk,
        )
    )

# Test imports when run standalone
if __name__ == "__main__":
    print(f"IS_CMK_25_OR_LATER: {IS_CMK_25_OR_LATER}")
    if IS_CMK_25_OR_LATER:
        print("✓ Successfully imported v1 API")
        print(f"✓ Rule spec created: {rule_spec_robotmk_bakery.name}")
    elif LEGACY_API_AVAILABLE:
        print("✓ Successfully imported legacy API")
        print("✓ Rule spec will be registered in GUI context")
    else:
        print("✗ No API available (likely standalone execution without GUI)")
    print("File loads successfully!")
