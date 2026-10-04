"""Keep the recorded original-bytecode evidence tied to its exact inputs."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class TraceProvenanceTests(unittest.TestCase):
    def test_fixture_and_driver_hashes_match_recorded_provenance(self):
        metadata = json.loads((ROOT / "tests/fixtures/java_combat.json").read_text())
        for key, name in [
            ("harness_sha256", "tools/CombatOracle.java"),
            ("scenarios_sha256", "tests/fixtures/combat_scenarios.tsv"),
            ("fixture_sha256", "tests/fixtures/java_combat.tsv"),
        ]:
            self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), metadata[key], name)
        lines = (ROOT / "tests/fixtures/java_combat.tsv").read_text().splitlines()
        self.assertEqual(sum(bool(line) and not line.startswith("#") for line in lines), metadata["rows"])
        self.assertEqual(len(metadata["action_queues"]), metadata["rows"])


if __name__ == "__main__":
    unittest.main()
