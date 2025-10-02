#!/usr/bin/python
# -*- encoding: utf-8; py-indent-offset: 4 -*-

# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

try:
    from cmk.utils import version
    cmk_version = version.get_general_version_infos()['version']
except ImportError:
    from cmk.utils.version import get_general_version_infos
    cmk_version = get_general_version_infos()['version']



def define_metrics_v1():
    metric_runner_runtime = Metric(
        name="runner_runtime",
        title=Title('Runtime (total)'),
        unit=Unit(TimeNotation()),
        color=Color.DARK_PINK,
    )
    metric_runner_runtime_robotmk = Metric(
        name="runner_runtime_robotmk",
        title=Title('Runtime (Robotmk code)'),
        unit=Unit(TimeNotation()),
        color=Color.YELLOW,  
    )
    metric_runner_runtime_suites = Metric(
        name="runner_runtime_suites",
        title=Title('Runtime (suite execution)'),
        unit=Unit(TimeNotation()),
        color=Color.DARK_PINK, 
    )
    metric_runner_cache_time = Metric(
        name="runner_cache_time",
        title=Title('Cache Time'),
        unit=Unit(TimeNotation()),
        color=Color.BLUE,
    )
    metric_runner_execution_interval = Metric(
        name="runner_execution_interval",
        title=Title('Execution interval'),
        unit=Unit(TimeNotation()),
        color=Color.DARK_GREEN,
    )

    # Suites counter
    metric_suites_total = Metric(
        name="suites_total",
        title=Title('Suites Total'),
        unit=Unit(DecimalNotation(""), StrictPrecision(0)),
        color=Color.WHITE, 
    )
    metric_suites_stale = Metric(
        name="suites_stale",
        title=Title('Suites Stale'),
        unit=Unit(DecimalNotation(""), StrictPrecision(0)),
        color=Color.LIGHT_BROWN,  
    )
    metric_suites_nonstale = Metric(
        name="suites_nonstale",
        title=Title('Suites Non-Stale'),
        unit=Unit(DecimalNotation(""), StrictPrecision(0)),
        color=Color.LIGHT_GREEN,
    )
    metric_suites_fatal = Metric(
        name="suites_fatal",
        title=Title('Suites Fatal'),
        unit=Unit(DecimalNotation(""), StrictPrecision(0)),
        color=Color.DARK_RED,
    )

    graph_headroom_usage = Graph(
        name='robotmk_headroom_usage',
        title=Title('Robotmk Runner - Runtime Headroom Usage'),
        minimal_range=MinimalRange(0, 
            Product(
                factors=["runner_cache_time", Constant(value=1.05, title=Title(""), color=Color.BLACK, unit=Unit(DecimalNotation("")))], 
                title=Title(""), color=Color.BLACK, unit=Unit(DecimalNotation("")) 
            ) 
        ), 
        simple_lines=[
            "runner_cache_time",
            "runner_execution_interval",
            "runner_runtime_robotmk",
            "runner_runtime_suites",
            "runner_runtime"
        ],
    )

    graph_suite_state = Graph(
        name='robotmk_suite_state',
        title=Title('Robot Framework Suites'),
        compound_lines=[
            "suites_stale",
            "suites_nonstale",
            "suites_fatal",
        ],
        simple_lines=[
            "suites_total"
        ],
    )

def define_metrics_legacy():
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


# switch: 2.4, 2.3, 2.2
if cmk_version.startswith(('2.4')):
    from cmk.graphing.v1 import Title
    from cmk.graphing.v1.graphs import Graph, MinimalRange
    from cmk.graphing.v1.metrics import Color, DecimalNotation, Metric, Unit, TimeNotation, StrictPrecision, Product, Constant
    from cmk.graphing.v1.perfometers import Closed, FocusRange, Open, Perfometer
    define_metrics_v1()
elif cmk_version.startswith(('2.2','2.3')):
    define_metrics_legacy()
else: 
    raise ValueError(f"Unsupported Checkmk version: {cmk_version}")
