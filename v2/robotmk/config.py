from __future__ import annotations

from collections.abc import Iterator, Mapping, Sequence
from dataclasses import dataclass
from enum import Enum
from pathlib import Path
from typing import Literal

from pydantic import BaseModel, TypeAdapter


def parse_config(path: Path) -> ConfigSystemPython | ConfigRCC:
    config = TypeAdapter(ConfigSystemPython | ConfigRCC).validate_json(path.read_text())
    # mypy somehow doesn't understand TypeAdapter.validate_json
    assert isinstance(config, ConfigSystemPython | ConfigRCC)
    return config


class ConfigSystemPython(BaseModel, frozen=True):
    environment: Literal["system_python"]
    working_directory: Path
    results_directory: Path
    suites: Mapping[str, SystemPythonSuiteConfig]

    def suite_specifications(self) -> Iterator[SuiteSpecification]:
        yield from (
            SuiteSpecification(
                name=suite_name,
                config=suite_config,
                rcc_env=None,
                working_directory=self.working_directory,
                results_directory=self.results_directory,
            )
            for suite_name, suite_config in self.suites.items()
        )


class ConfigRCC(BaseModel, frozen=True):
    environment: RCCConfig
    working_directory: Path
    results_directory: Path
    suites: Mapping[str, RCCSuiteConfig]

    def suite_specifications(self) -> Iterator[SuiteSpecification]:
        yield from (
            SuiteSpecification(
                name=suite_name,
                config=suite_config,
                rcc_env=RCCEnvironmentSpec(
                    binary_path=self.environment.rcc_binary_path,
                    robot_yaml_path=suite_config.robot_yaml_path,
                ),
                working_directory=self.working_directory,
                results_directory=self.results_directory,
            )
            for suite_name, suite_config in self.suites.items()
        )


class RCCConfig(BaseModel, frozen=True):
    rcc_binary_path: Path


class SuiteConfig(BaseModel, frozen=True):
    execution_interval_seconds: int
    robot_target: Path
    variants: Sequence[Variant]
    retry_strategy: RetryStrategy
    session: UserSessionConfig | None


class SystemPythonSuiteConfig(SuiteConfig, frozen=True):
    ...


class RCCSuiteConfig(SuiteConfig, frozen=True):
    robot_yaml_path: Path


class Variant(BaseModel, frozen=True):
    variablefile: Path | None
    argumentfile: Path | None


class RetryStrategy(Enum):
    INCREMENTAL = "incremental"
    COMPLETE = "complete"


class UserSessionConfig(BaseModel, frozen=True):
    user_name: str


@dataclass(frozen=True)
class SuiteSpecification:
    name: str  # ambiguous, since Robot Framework also provides names.
    config: SuiteConfig
    rcc_env: RCCEnvironmentSpec | None
    working_directory: Path
    results_directory: Path


@dataclass(frozen=True)
class RCCEnvironmentSpec:
    binary_path: Path
    robot_yaml_path: Path
