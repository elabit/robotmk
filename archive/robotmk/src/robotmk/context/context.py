from robotmk.context.abstract import AbstractContext


class ContextFactory:
    def __init__(self, contextname: str, log_level: str | None) -> None:
        self.contextname = contextname
        self._log_level = log_level

    def get_context(self) -> AbstractContext:
        if self.contextname == "agent":
            from .agent.agent import AgentContext

            return AgentContext()
        elif self.contextname == "specialagent":
            from .specialagent.specialagent import SpecialAgentContext

            return SpecialAgentContext()
        elif self.contextname == "suite":
            from .suite.suite import SuiteContext

            return SuiteContext()
        raise NotImplementedError()
