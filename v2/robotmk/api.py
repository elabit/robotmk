from pydantic import BaseModel


class Result(BaseModel, frozen=True):
    suite_name: str
    xml: str
