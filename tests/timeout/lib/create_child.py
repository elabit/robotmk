import time
import pathlib


def spawn() -> None:
    time.sleep(10)


def setup(path: pathlib.Path) -> None:
    path.touch(exist_ok=False)


def teardown(path: pathlib.Path) -> None:
    path.unlink(missing_ok=False)
