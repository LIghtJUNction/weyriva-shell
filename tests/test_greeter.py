from pathlib import Path
import unittest


ROOT = Path(__file__).resolve().parents[1]


class GreeterRuntimeTests(unittest.TestCase):
    def test_cage_greeter_uses_a_fullscreen_toplevel(self) -> None:
        source = (ROOT / "greeter/shell.qml").read_text(encoding="utf-8")

        self.assertIn("FloatingWindow {", source)
        self.assertIn("visible: true", source)
        self.assertIn("fullscreen: true", source)
        self.assertIn('title: "Weyriva Greeter"', source)
        self.assertNotIn("PanelWindow {", source)


if __name__ == "__main__":
    unittest.main()
