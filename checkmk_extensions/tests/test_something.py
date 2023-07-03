#!/usr/bin/env python3

from cmk.base.plugins.agent_based.agent_based_api.v1 import State


def test_something() -> None:
    assert State.OK is State.OK
