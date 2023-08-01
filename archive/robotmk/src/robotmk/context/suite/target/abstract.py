from abc import ABC, abstractmethod


class Target(ABC):
    """A Target defines the environment where a suite gets executed.

    It's the abstraction of either
    - a local Robot suite or ("target: local")
    - an API call to an external platform ("target: remote") like Robocorp or Kubernetes
    """

    def __init__(self, suiteuname: str, piggybackhost: str):
        self.suiteuname = suiteuname
        self.piggybackhost = piggybackhost

    @abstractmethod
    def run(self):
        """Abstract method to run a suite/target."""

    @abstractmethod
    def output(self):
        """Abstract method to get the output of a suite/target."""
