import subprocess


def spawn(file_name: str) -> None:
    with subprocess.Popen(["python", "-c", "import time; time.sleep(3)"]) as process:
        with open(file_name, "w", encoding="utf-8") as file:
            print(process.pid, file=file)
        process.wait(3)
