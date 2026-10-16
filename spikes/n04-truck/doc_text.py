"""Read rendered public rustdoc, excluding navigation and source implementations."""

from html.parser import HTMLParser
from pathlib import Path
import sys

sys.stdout.reconfigure(encoding="utf-8")


class Text(HTMLParser):
    def __init__(self):
        super().__init__()
        self.active = False
        self.parts = []

    def handle_starttag(self, tag, attrs):
        if tag == "main":
            self.active = True
        if self.active and tag in ("p", "section", "h2", "h3", "pre", "li"):
            self.parts.append("\n")

    def handle_endtag(self, tag):
        if tag == "main":
            self.active = False

    def handle_data(self, data):
        if self.active and not data.startswith('{"'):
            self.parts.append(data)


for name in sys.argv[1:]:
    path = Path(__file__).parent / "target/doc" / name
    parser = Text()
    parser.feed(path.read_text(encoding="utf-8"))
    print(name, "\n", "".join(parser.parts))
