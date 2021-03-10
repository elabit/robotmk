#!/usr/bin/python
# -*- encoding: utf-8; py-indent-offset: 4 -*-

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

metric_info['runner_runtime'] = {
    'title': _('Runtime (total)'),
    'unit': 's',
    'color': '#ff2377',
}
metric_info['runner_runtime_robotmk'] = {
    'title': _('Runtime (Robotmk code)'),
    'unit': 's',
    'color': '#fcce6a',
}
metric_info['runner_runtime_suites'] = {
    'title': _('Runtime (suite execution)'),
    'unit': 's',
    'color': '#c04080',
}
metric_info['runner_cache_time'] = {
    'title': _('Cache Time'),
    'unit': 's',
    'color': '#2e6ec8',
}
metric_info['runner_execution_interval'] = {
    'title': _('Execution interval'),
    'unit': 's',
    'color': '#209c58',
}

# Suites counter
metric_info['suites_total'] = {
    'title': _('Suites Total'),
    'unit': 'count',
    'color': '#eeeade',
}
metric_info['suites_stale'] = {
    'title': _('Suites Stale'),
    'unit': 'count',
    'color': '#b78583',
}
metric_info['suites_nonstale'] = {
    'title': _('Suites Non-Stale'),
    'unit': 'count',
    'color': '#72be6c',
}
metric_info['suites_fatal'] = {
    'title': _('Suites Fatal'),
    'unit': 'count',
    'color': '#b7241d',
}

graph_info['robotmk_headroom_usage'] = {
    "title":
    _("Robotmk Runner - Runtime Headroom Usage"),
    "metrics": [
        ("runner_cache_time", "area"),
        ("runner_execution_interval", "area"),
        ("runner_runtime_robotmk", "area"),
        ("runner_runtime_suites", "stack"),
        ("runner_runtime", "line"),
    ],
    "optional_metrics": ["runner_execution_interval"],
    "range": (0, "runner_cache_time,1.05,*"),
    "scalars": [
        "runner_runtime:warn",
        "runner_runtime:crit",
    ]
}

graph_info['robotmk_suite_state'] = {
    "title":
    _("Robot Framework Suites"),
    "metrics": [
        ("suites_stale", "area"),
        ("suites_nonstale", "stack"),
        ("suites_fatal", "stack"),
        ("suites_total", "line"),
    ],
}

# If the runtime is ok, the perfometer shows decent colors.
perfometer_info.append({
    "type":
    "linear",
    "condition":
    "runner_runtime,runner_runtime:warn,<",
    "segments": [
        "runner_runtime#a8665d",
        "runner_cache_time,runner_runtime,-#bcbdbc",
    ],
    "total":
    "runner_cache_time",
    "label": ("runner_runtime", "s")
})
perfometer_info.append({
    "type":
    "linear",
    "condition":
    "runner_runtime,runner_runtime:warn,>=",
    "segments": [
        "runner_runtime",
        "runner_cache_time,runner_runtime,-#409c58",
    ],
    "total":
    "runner_cache_time",
    "label": ("runner_runtime", "s")
})
