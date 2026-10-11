"""Keep the recorded original-bytecode evidence tied to its exact inputs."""
import hashlib
import json
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[1]


class TraceProvenanceTests(unittest.TestCase):
    def test_action_provenance(self):
        self.check_provenance("java_combat", "combat_scenarios")

    def test_turn_provenance(self):
        self.check_provenance("java_turns", "turn_scenarios")

    def test_jaw_worm_provenance(self):
        self.check_provenance("java_jaw_worm", "jaw_worm_scenarios")

    def test_encounter_provenance(self):
        self.check_provenance("java_encounters", "encounter_scenarios")

    def check_provenance(self, fixture, scenarios):
        metadata = json.loads((ROOT / f"tests/fixtures/{fixture}.json").read_text())
        for key, name in [
            ("harness_sha256", "tools/CombatOracle.java"),
            ("scenarios_sha256", f"tests/fixtures/{scenarios}.tsv"),
            ("fixture_sha256", f"tests/fixtures/{fixture}.tsv"),
        ]:
            self.assertEqual(hashlib.sha256((ROOT / name).read_bytes()).hexdigest(), metadata[key], name)
        lines = (ROOT / f"tests/fixtures/{fixture}.tsv").read_text().splitlines()
        self.assertEqual(sum(bool(line) and not line.startswith("#") for line in lines), metadata["rows"])
        self.assertEqual(len(metadata["action_queues"]), metadata["rows"])


if __name__ == "__main__":
    unittest.main()
