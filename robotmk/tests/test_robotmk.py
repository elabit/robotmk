from robotmk.main import Robotmk, DEFAULTS
import pytest
import os


def test_robotmk_no_context_in_env():
    """Test that Robotmk throws an error if created without any context in environment."""
    match_msg = (
        "No context given on CLI or set by environment variable ROBOTMK_common_context."
    )
    with pytest.raises(ValueError, match=match_msg):
        robotmk = Robotmk()


def test_robotmk_local_context():
    """Test that Robotmk has agent context when set in environment."""
    os.environ["ROBOTMK_common_context"] = "agent"
    robotmk = Robotmk()
    assert robotmk.config.get("common.context") == "agent"


def test_robotmk_suite_context():
    """Test that Robotmk has suite context when set in environment."""
    os.environ["ROBOTMK_common_context"] = "suite"
    robotmk = Robotmk()
    assert robotmk.config.get("common.context") == "suite"


def test_robotmk_specialagent_context():
    """Test that Robotmk has specialagent context when set in environment."""
    os.environ["ROBOTMK_common_context"] = "specialagent"
    robotmk = Robotmk()
    assert robotmk.config.get("common.context") == "specialagent"
