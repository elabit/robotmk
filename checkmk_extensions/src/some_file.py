#!/usr/bin/env python3

from cmk.gui.valuespec import Dictionary


def my_fun() -> type[Dictionary]:
    return Dictionary
