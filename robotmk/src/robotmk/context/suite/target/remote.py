from .abstract import Target
from ..strategies import RunStrategy
from robotmk.logger import RobotmkLogger


class RemoteTarget(Target):
    def __init__(self, suiteuname: str, config: dict, logger: RobotmkLogger):
        super().__init__(suiteuname, config, logger)

    def run(self):
        print("NOT YET IMPLEMENTED")
        pass

    def output(self):
        # return a dummy dict from a dummy REST API call
        # TODO: implement REST API Call to Robocorp to fetch the output
        return {
            "suiteuname": self.suiteuname,
            "uuid": "5227a230696f40939a51d60dc1cbdd29",
            "rc": 1,
            "start_time": "2023-04-25T16:58:34.573492+02:00",
            "end_time": "2023-04-25T16:58:56.965265+02:00",
            "runtime": 22.391773,
            "piggybackhost": self.piggybackhost,
            "output": {
                "html": {
                    "path": "/home/simonmeggle/Documents/01_dev/rmkv2/agent/log/robotmk/robotframework/rf_suite_default_1682434706_2aa6fe6d.html"
                },
                "xml": {
                    "path": "/home/simonmeggle/Documents/01_dev/rmkv2/agent/log/robotmk/robotframework/rf_suite_default_1682434706_2aa6fe6d.xml",
                    "base64": "eNrlVU1PGzEQvfMrLB9QWynrbIBAIVmUfqAiAUWQnitnd7qx4o+V7U3g39fj3YSEBFSp5dST7fHMe/PGY3tw/qAkmYN1wughTZMuJaBzUwhdDumP8UXnhJ5newNrJsaTEjRY7o0d0jtAQz9JeUre3T76qdHkIEm7SZ+EmRS6fnhPlwFQDGmv2zvoHvaOSNo/PTo5PeonHw+PKbEVH1Jva6DE5VNQfJXLIQ28rhYeiAjxLqVEcwVhhrZR8De1zcOaTY0C5oQyWkFZSmBfTF4r0N6xbvqzgDmzajbvMR7S8SxKcaxFecbReaIZczdz/4aFecRKog0ZPTj/RNjxK85roYXikqA/xSoGWx8jZovW48qUZGzIZ6OdkYA+E8vt45B+qoX0lxqduS2zu3hgqIEsuCPwAHkdziEZMNzdGxQmzwKWI34KpBRz0ESBcyF54k005g1FiEDfUCbPfe1IMwzp7ej+nuLKei8wsx0HnPaxm4rX9llAZrPFusQLLuRuYZgI7jZZxyouhJ/u0MB1QUzlQyNxKR8Jlz60FRE+RPLSrTQpVxLMLshQ1e4UQ49KmIMMeY0ur2g2cg4sAn+11tgBCxDbxYmuf1uc47XivA1+9uHb+PrqA9mX/sxVXJNcchfgFdgS6H7pz2IDTUMDTSBU10Jn2UixwhZcLUNNo3+RIAxDHIzExdQuZ+voGhadRkikuIFFq+t0A2Arq1/YF7iD6re4Ji9ytT2xImvXz9g2z3UTcbcOI4t1Hd9l8XY6kGtdB5L9nzoGrEkOrwa+AO2zMAZVyfDTkOanUlxoEh/fl96vrSt0w0ZrF6ZZSV45aC1pcx0j6NvACedFjsK88Vy2RlLFwoV/GeuNccTNRIWWbCRlfOJdU5RYkjY0PHNxGYd1klfwdv+zWTOuKP4MZOsjbWGSZ/kuU2MbBQA883jET7N4sNneb5Vz1Zw=",
                },
            },
        }
