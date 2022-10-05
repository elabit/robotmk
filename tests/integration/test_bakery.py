#!/bin/env python3
import sys
import os
from pathlib import Path

import pytest
from cmk.base.cee.plugins.bakery.bakery_api.v1 import FileGenerator, OS, PluginConfig

sys.path.insert(
    0,
    Path(os.getenv("OMD_ROOT"))
    .joinpath("local/share/check_mk/agents/bakery")
    .as_posix(),
)
import robotmk


bakery_args = [
    (
        {
            "execution_mode": (
                "agent_serial",
                {
                    "suites": [
                        {
                            "path": "sleep",
                            "failed_handling": {
                                "max_iterations": 2,
                                "strategy": {
                                    "name": "incremental",
                                    "rerun_selection": {
                                        "suite": ["testsuite1", "testsuite2"],
                                        "test": ["test1", "test2"],
                                        "include": ["tag1", "tag2"],
                                        "exclude": ["tag3", "tag4"],
                                    },
                                },
                            },
                        }
                    ],
                    "cache_time": 960,
                    "execution_interval": 900,
                },
            ),
            "agent_output_encoding": "zlib_codec",
            "transmit_html": False,
            "log_level": "INFO",
            "log_rotation": 7,
            "dirs": {},
        },
        [
            "global:",
            "  execution_mode: agent_serial",
            "  agent_output_encoding: zlib_codec",
            "  transmit_html: false",
            "  log_level: INFO",
            "  log_rotation: 7",
            "  cache_time: 960",
            "  execution_interval: 900",
            "suites:",
            "  sleep:",
            "    path: sleep",
            "    failed_handling:",
            "      max_iterations: 2",
            "      strategy:",
            "        name: incremental",
            "        rerun_selection:",
            "          exclude:",
            "          - tag3",
            "          - tag4",
            "          include:",
            "          - tag1",
            "          - tag2",
            "          suite:",
            "          - testsuite1",
            "          - testsuite2",
            "          test:",
            "          - test1",
            "          - test2",
        ],
    )
]

fixture = [(list(robotmk.get_robotmk_files(i[0])), i[1]) for i in bakery_args]


@pytest.mark.parametrize("rmk_files, expect_lines", fixture)
def test_robotmk_yml(rmk_files, expect_lines):
    for operating_system in [OS.LINUX, OS.WINDOWS]:
        for rmk_file in rmk_files:
            if rmk_file.base_os is operating_system and isinstance(
                rmk_file, PluginConfig
            ):

                assert str(rmk_file.target == "robotmk.yml")
                lines = list(
                    filter(
                        lambda x: not x.startswith("#") and len(x) > 0, rmk_file.lines
                    )
                )
                for i, l in enumerate(lines):
                    assert l == expect_lines[i]
                pass
