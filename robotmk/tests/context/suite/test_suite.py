import yaml
from robotmk.main import Robotmk
import os

cwd = os.path.dirname(__file__)
robotmk_yml = os.path.join(cwd, "robotmk.yml")
robotmk_env = os.path.join(cwd, "robotmk.env")


def test_suite_context():
    """Tests if a Robotmk object can be created for suite context."""
    robotmk = Robotmk(contextname="suite", log_level=None, ymlfile=robotmk_yml)
    assert robotmk.config.get("common.suiteuname") == "suite_ospython"
