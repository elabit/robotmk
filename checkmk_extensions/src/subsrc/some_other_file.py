#!/usr/bin/env python3

from cmk.base.plugins.agent_based.agent_based_api.v1 import State

from robotmk.config.config import DIR_SUBPATHS


def _state() -> State:
    return State.OK


def whatever() -> str:
    return str(DIR_SUBPATHS["cfgdir"])
