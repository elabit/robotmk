from schema import Schema, SchemaError, And, Or, Use, Optional, Regex
import yaml


class RobotmkConfigSchema:
    """This class is used to validate the Robotmk config."""

    subdict_schema = Schema({str: str})
    failed_handling_schema = Schema({str: str})
    schema = Schema(
        {
            "common": {
                "context": Or("suite", "specialagent", "local"),
                # "suite": str,
                "cfgdir": str,
                "logdir": str,
                "tmpdir": str,
                "resultdir": str,
                "log_level": Or("debug", "info", "warning", "error", "critical"),
                "log_retention": int,
                Optional("suite"): str,
                Optional("k8s_auth"): {"user": str, "password": str, "url": str},
                Optional("robocorp_auth"): {"user": str, "password": str, "url": str},
            },
            "suites": {
                str: {
                    "path": str,
                    Optional("tag"): str,
                    Optional("piggybackhost"): str,
                    "run": Or(
                        {
                            "mode": "default",
                            "target": "local",
                            "rcc": Or(True, False),
                        },
                        {
                            "mode": Or(
                                "windows-1desktop", "windows-ndesktop", "linux-ndesktop"
                            ),
                            "target": "local",
                            "rcc": Or(True, False),
                            "user": str,
                            "password": str,
                        },
                    ),
                    # Optional("robot_params"): And(dict, Use(subdict_schema.validate)),
                    # Optional("failed_handling"): And(
                    #     dict, Use(failed_handling_schema.validate)
                    # ),
                    Optional("scheduling"): {"interval": int, "allow_overlap": bool},
                }
            },
        },
        ignore_extra_keys=True,
    )

    def __init__(self, data) -> None:
        self.data = data
        self.valid = None

    def validate(self) -> None:
        """Validate the config, set the valid property and return it."""
        try:
            self.valid = self.schema.validate(self.data)
        except SchemaError as e:
            self.valid = False
            self.error = e
        return self.valid
