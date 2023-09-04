import enum
from datetime import datetime
from typing import Annotated, TypeVar

import xmltodict
from pydantic import BaseModel, BeforeValidator, Field, PlainValidator


def parse_datetime(raw: str) -> datetime:
    return datetime.strptime(raw, "%Y%m%d %H:%M:%S.%f")


T = TypeVar("T")


def parse_children(raw: T | list[T]) -> list[T]:
    return raw if isinstance(raw, list) else [raw]


DateTime = Annotated[datetime, PlainValidator(parse_datetime)]


class XML(BaseModel, frozen=True):
    ...


class Outcome(enum.Enum):
    FAIL = "FAIL"
    PASS = "PASS"
    SKIP = "SKIP"
    NOT_RUN = "NOT RUN"


class Status(XML, frozen=True):
    status: Outcome = Field(alias="@status")
    starttime: DateTime = Field(alias="@starttime")
    endtime: DateTime = Field(alias="@endtime")


class Test(XML, frozen=True):
    id_: str = Field(alias="@id")
    name: str = Field(alias="@name")
    line: int = Field(alias="@line")
    status: Status


class Suite(XML, frozen=True):
    id_: str = Field(alias="@id")
    name: str = Field(alias="@name")
    suite: Annotated[list["Suite"], BeforeValidator(parse_children)] = Field(default=[])
    test: Annotated[list[Test], BeforeValidator(parse_children)] = Field(default=[])


class Generator(XML, frozen=True):
    generator: str = Field(alias="@generator")
    generated: DateTime = Field(alias="@generated")
    rpa: bool = Field(alias="@rpa")
    # TODO: Ensure schemaversions other than `4` work.  # pylint: disable=fixme
    schemaversion: int = Field(alias="@schemaversion")
    suite: Annotated[list[Suite], BeforeValidator(parse_children)] = Field(default=[])
    errors: object  # TODO: Gracefully handle errors  # pylint: disable=fixme


class Rebot(XML, frozen=True):
    robot: Generator


def parse_rebot(xml: str) -> Rebot:
    return Rebot.model_validate(xmltodict.parse(xml))
