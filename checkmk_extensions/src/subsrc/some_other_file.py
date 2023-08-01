#!/usr/bin/env python3

from cmk.base.plugins.agent_based.agent_based_api.v1 import State


def _state() -> State:
    return State.OK
