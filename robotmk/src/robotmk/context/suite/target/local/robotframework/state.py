import json
from pathlib import Path
import base64
import zlib
from robotmk import __version__


class RFState:
    def __init__(self, target) -> None:
        self.target = target

    @property
    def statedata(self) -> dict:
        return {
            "suiteuname": self.target.suiteuname,
            "uuid": self.target.uuid,
            "rc": self.target.rc,
            "start_time": self._start_time.isoformat(),
            "end_time": self._end_time.isoformat(),
            "runtime": self._runtime.total_seconds(),
            "piggybackhost": self.target.config.get("suitecfg.piggybackhost", None),
            "output": {
                "html": {
                    "path": self.target.log_html_fullpath,
                },
                "xml": {
                    "path": self.target.output_xml_fullpath,
                    "base64": self.encode(
                        self.read_file(self.target.output_xml_fullpath)
                    ),
                },
                "console": self.target.console_results,
            },
            "robotmk_version": __version__,
        }

    def write(self) -> None:
        """Writes the console output to logdir and result JSON file to outputdir."""
        self.write_console_log()
        self.write_result_json()

    def timer_start(self) -> None:
        self._start_time = self.target._get_now_as_dt()

    def timer_stop(self) -> None:
        self._end_time = self.target._get_now_as_dt()
        self._runtime = self._end_time - self._start_time

    def write_console_log(self) -> None:
        for k, result in self.target.console_results.items():
            # HEREIWAS:
            data = json.dumps(result, indent=4)
            filename = (
                Path(self.target.outputdir)
                / f"{self.target.output_filename}-{int(k)}.txt"
            )
            self.write_data2file(data, filename)
            pass

    def write_result_json(self) -> None:
        """ """

        try:
            Path(self.target.statefile_fullpath).parent.mkdir(
                parents=True, exist_ok=True
            )
            with open(self.target.statefile_fullpath, "w", encoding="utf-8") as outfile:
                json.dump(self.statedata, outfile, indent=2, sort_keys=False)
        except IOError as e:
            # Error gets logged, will come to light by staleness check
            pass

    def write_data2file(self, data: str, filename: str):
        """Generic function to write data to a file."""
        try:
            with open(filename, "w") as f:
                f.write(data)
        except Exception as e:
            pass
            # TODO
            # raise RobotmkError("Failed to write data to file: %s" % e)

    def read_file(self, path, default=None):
        content = None
        try:
            with open(path, "r", encoding="utf-8") as file:
                content = file.read()
                if len(content) == 0:
                    # TODO: logging
                    # self.logwarn(
                    #     "File %s has no content, using defaults (%s)"
                    #     % (path, str(default))
                    # )
                    content = default
        except Exception as e:
            # TODO: logging
            # self.logwarn(
            #     "Error while reading %s (%s); using default (%s)"
            #     % (path, e, str(default))
            # )
            content = default
        return content

    def encode(self, data, encoding="zlib_codec"):
        # Caveat: to keep the zlib stream integrity, it must be converted to a
        # "safe" stream afterwards.
        # Reason: if there is a byte in the zlib stream which is a newline byte
        # by accident, Checkmk splits the byte string at this point - the
        # byte gets lost, stream integrity bungled.
        # Even if base64 blows up the data, this double encoding still saves space:
        # in:      692800 bytes  100    %
        # zlib:      4391 bytes    0,63 % -> compression 99,37%
        # base64:    5856 bytes    0,85 % -> compression 99,15%

        #    1. encode in UTF8
        #   2. compress with zlib
        #  3. encode with base64

        if encoding == "base64_codec":
            data_bytes = data.encode("utf-8")
            data_encoded = base64.b64encode(data_bytes)
            data_utf8 = data_encoded.decode("utf-8")
        elif encoding == "zlib_codec":
            data_bytes = data.encode("utf-8")
            data_zlib = zlib.compress(data_bytes, 9)
            data_encoded = base64.b64encode(data_zlib)
            data_utf8 = data_encoded.decode("utf-8")
        elif encoding == "utf_8":
            # nothing to do, already in utf8 = string
            data_utf8 = data
        else:
            # TODO: Catch the exception! (wrong encoding)!
            pass
        return data_utf8
