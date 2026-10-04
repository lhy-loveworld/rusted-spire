"""Execute controlled action traces from a locally owned game JAR.

Only numeric/card-state traces and provenance are published, never game code.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def sha256(path):
    with path.open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--jar", type=Path, required=True)
    parser.add_argument("--java-home", type=Path, default=os.environ.get("JAVA_HOME"))
    parser.add_argument("--scenarios", type=Path, default=ROOT / "tests/fixtures/combat_scenarios.tsv")
    parser.add_argument("--output", type=Path, default=ROOT / "tests/fixtures/java_combat.tsv")
    args = parser.parse_args()
    jar, scenarios = args.jar.resolve(strict=True), args.scenarios.resolve(strict=True)
    harness = ROOT / "tools/CombatOracle.java"
    def executable(name):
        return str(args.java_home / "bin" / name) if args.java_home else name
    # The game writes diagnostic/display files relative to cwd. Never use the
    # game installation or the user's workspace as the oracle working directory.
    with tempfile.TemporaryDirectory(prefix="rusted-spire-combat-") as directory:
        compiled = subprocess.run([executable("javac"), "-proc:none", "-cp", str(jar), "-d", directory, str(harness)],
                                  cwd=directory, capture_output=True, text=True, timeout=120)
        if compiled.returncode:
            raise RuntimeError(compiled.stdout + compiled.stderr)
        result = subprocess.run([executable("java"), "-cp", os.pathsep.join([directory, str(jar)]),
                                 "CombatOracle", str(scenarios)], cwd=directory,
                                capture_output=True, text=True, timeout=120)
        if result.returncode:
            raise RuntimeError(result.stdout + result.stderr)
    rows = [line.removeprefix("TRACE\t") for line in result.stdout.splitlines() if line.startswith("TRACE\t")]
    expected = sum(bool(line.strip()) and not line.startswith("#") for line in scenarios.read_text().splitlines())
    if len(rows) != expected or any(len(row.split("\t")) != 17 for row in rows):
        raise RuntimeError(f"invalid oracle output: {len(rows)} rows, expected {expected}")
    version = next(line.removeprefix("VERSION\t") for line in result.stdout.splitlines() if line.startswith("VERSION\t"))
    data = "# case step phase hp block energy hand draw discard exhaust enemy_hp enemy_block player_powers enemy_powers card_rng shuffle_rng choices\n" + "\n".join(rows) + "\n"
    jdk = subprocess.run([executable("java"), "-version"], check=True, capture_output=True, text=True)
    metadata = {
        "scope": "Original JAR bytecode for card use, queued actions, damage, powers, pile movement and choices; controlled actor setup and queue driver, mock rendering. Not full game-loop or turn/AI parity.",
        "game_version": version, "jar_sha256": sha256(jar),
        "java_version": (jdk.stdout + jdk.stderr).strip(),
        "harness_sha256": sha256(harness), "scenarios_sha256": sha256(scenarios),
        "fixture_sha256": hashlib.sha256(data.encode()).hexdigest(), "rows": len(rows),
        "action_queues": [line.split("\t")[1:] for line in result.stdout.splitlines() if line.startswith("QUEUE\t")],
    }
    args.output.parent.mkdir(parents=True, exist_ok=True)
    args.output.write_text(data)
    args.output.with_suffix(".json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(f"Wrote {len(rows)} original-bytecode snapshots to {args.output}")


if __name__ == "__main__":
    main()
