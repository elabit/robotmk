from pydantic import BaseModel


class Result(BaseModel, frozen=True):
    xml: str
