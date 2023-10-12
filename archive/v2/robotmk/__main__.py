from collections.abc import Iterable
from pathlib import Path

from apscheduler.schedulers.blocking import BlockingScheduler  # type: ignore[import]

from robotmk.cli import parse_arguments
from robotmk.config import ConfigRCC, ConfigSystemPython, parse_config
from robotmk.scheduling import (
    schedule_suites,
    suite_result_file,
    suite_results_directory,
)


def _main() -> None:
    arguments = parse_arguments()
    config = parse_config(arguments.config_path)
    _setup(config)
    schedule_suites(config, BlockingScheduler()).start()


def _setup(config: ConfigSystemPython | ConfigRCC) -> None:
    config.working_directory.mkdir(
        parents=True,
        exist_ok=True,
    )
    (suite_results_dir := suite_results_directory(config.results_directory)).mkdir(
        parents=True,
        exist_ok=True,
    )
    _clean_up_results_directory_atomic(
        suite_results_dir=suite_results_dir,
        configured_suites=config.suites,
        intermediate_path_for_move=config.working_directory / "deprecated_result",
    )


def _clean_up_results_directory_atomic(
    *,
    suite_results_dir: Path,
    configured_suites: Iterable[str],
    intermediate_path_for_move: Path,
) -> None:
    for unwanted_result_file in set(suite_results_dir.iterdir()) - {
        suite_result_file(suite_results_dir, suite_name)
        for suite_name in configured_suites
    }:
        unwanted_result_file.replace(intermediate_path_for_move)
    intermediate_path_for_move.unlink(missing_ok=True)


_main()
