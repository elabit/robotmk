#!/usr/bin/python
# -*- encoding: utf-8; py-indent-offset: 4 -*-

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

metric_info['plugin_runtime_total'] = {
    'title' : _('Plugin runtime (total)'),
    'unit' : 's',
    'color' : '#ff2377',
}
metric_info['plugin_runtime_robotmk'] = {
    'title' : _('Plugin runtime (Robotmk code)'),
    'unit' : 's',
    'color' : '#4488cc',
}
metric_info['plugin_runtime_suites'] = {
    'title' : _('Plugin runtime (suite execution time)'),
    'unit' : 's',
    'color' : '#c04080',
}
metric_info['plugin_cache_time'] = {
    'title' : _('Plugin Cache Time'),
    'unit' : 's',
    'color' : '#409c58',
}

# Suites counter
metric_info['suites_total'] = {
    'title' : _('Suites Total'),
    'unit' : '',
    'color' : '#66a887',
}
metric_info['suites_stale'] = {
    'title' : _('Suites Stale'),
    'unit' : '',
    'color' : '#a8665d',
}

graph_info['robotmk_cachetime_usage'] = {
    "title": _("Robotmk Plugin Cachetime Usage"),
    "metrics": [
        ("plugin_cache_time", "area"),
        ("plugin_runtime_robotmk", "area"),
        ("plugin_runtime_suites", "stack"),
        ("plugin_runtime_total", "line"),
    ],
    "range": (0, "plugin_cache_time"),
    "scalars": [
        "plugin_runtime_total:warn",
        "plugin_runtime_total:crit",
    ]
    }

graph_info['robotmk_suite_state'] = {
    "title": _("Robotmk Suites"),
    "metrics": [
        ("suites_total", "area"),
        ("suites_stale", "stack"),
    ],
    }

# If the runtime is ok, the perfometer shows decent colors. 
perfometer_info.append({
    "type": "linear",
    "condition": "plugin_runtime_total,plugin_runtime_total:warn,<",
    "segments": [
        "plugin_runtime_total#a8665d",
        "plugin_cache_time,plugin_runtime_total,-#bcbdbc",
        ],
    "total": "plugin_cache_time",
    "label": ("plugin_runtime_total", "s")
}) 
perfometer_info.append({
    "type": "linear",
    "condition": "plugin_runtime_total,plugin_runtime_total:warn,>=",
    "segments": [
        "plugin_runtime_total",
        "plugin_cache_time,plugin_runtime_total,-#409c58",
        ],
    "total": "plugin_cache_time",
    "label": ("plugin_runtime_total", "s")
}) 
