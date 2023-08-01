from .abstract import AbstractExecutor


class Sequencer(AbstractExecutor):
    def __init__(self, config, *args, **kwargs):
        super().__init__(config)

    def run(self):
        """Start the sequencer, runs all API calls and ends"""
        print("Sequencer.run()")
        pass
