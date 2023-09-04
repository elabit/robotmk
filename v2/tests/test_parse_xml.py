from robotmk import parse_xml


def test_parse_rebot() -> None:
    with open("v2/tests/rebot.xml", "r", encoding="utf-8") as file:
        content = file.read()
    parse_xml.parse_rebot(content)
