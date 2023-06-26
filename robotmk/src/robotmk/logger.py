import loguru
from abc import ABC, abstractmethod
from datetime import datetime


# TODO: concurrent writes to the log file
# e.g. RCC -> RF


class AbstractLogger(ABC):
    def __init__(self, log_level):
        self.log_level = log_level
        if not getattr(self, "logger", None):
            self.logger = loguru.logger
            # Disable the logger completely if log level is not set
            if self.log_level is None:
                self.logger.disable()
            else:
                self.logger.remove()  # Remove default configuration
                self.add_logger()

    @abstractmethod
    def add_logger(self):
        pass

    def debug(self, message, *args, **kwargs):
        self.logger.debug(message, *args, **kwargs)

    def info(self, message, *args, **kwargs):
        self.logger.info(message, *args, **kwargs)

    def warning(self, message, *args, **kwargs):
        self.logger.warning(message, *args, **kwargs)

    def error(self, message, *args, **kwargs):
        self.logger.error(message, *args, **kwargs)

    def critical(self, message, *args, **kwargs):
        self.logger.critical(message, *args, **kwargs)


class RobotmkLogger(AbstractLogger):
    def __init__(self, log_file_path, log_level="INFO"):
        self.log_file_path = log_file_path
        super().__init__(log_level)

    def add_logger(self):
        # Add the file sink
        self.logger.add(self.log_file_path, level=self.log_level)


# class JSONLogger(AbstractLogger):
#     """Logging into an internal message stack"""

#     def __init__(self, log_level="INFO"):
#         self.messages = []
#         self.logger.add(
#             {
#                 "sink": lambda msg: self.messages.append(
#                     self.message_to_dict(msg.record), log_level=log_level
#                 )
#             }
#         )

#     @staticmethod
#     def message_to_dict(record):
#         return {
#             **record,
#             "extra": {k: str(v) for k, v in record["extra"].items()},
#             "time": datetime.utcnow().isoformat() + "Z",
#         }
