#!/usr/bin/python
# -*- encoding: utf-8; py-indent-offset: 4 -*-

# SPDX-FileCopyrightText: © 2022 ELABIT GmbH <mail@elabit.de>
# SPDX-License-Identifier: GPL-3.0-or-later
# This file is part of the Robotmk project (https://www.robotmk.org)

from cmk.graphing.v1 import Title
from cmk.graphing.v1.graphs import Graph, MinimalRange
from cmk.graphing.v1.metrics import Color, DecimalNotation, Metric, Unit, TimeNotation, StrictPrecision, Product, Constant
from cmk.graphing.v1.perfometers import Closed, FocusRange, Open, Perfometer

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
