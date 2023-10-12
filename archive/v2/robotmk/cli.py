from argparse import ArgumentParser
from pathlib import Path

from pydantic import BaseModel


class Arguments(BaseModel, frozen=True):
    config_path: Path


def parse_arguments() -> Arguments:
    parser = ArgumentParser()
    parser.add_argument("config_path", type=Path)
    return Arguments.model_validate(vars(parser.parse_args()))
