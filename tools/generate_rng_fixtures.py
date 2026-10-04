"""Execute the local decompiled libGDX RNG; publish numeric results, not its source.

Run with the project's .venv Python. Requires java/javac (or --java-home).
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--java-home", type=Path, default=os.environ.get("JAVA_HOME"))
    parser.add_argument("--source", type=Path, default=ROOT / "decompiled/sources/com/badlogic/gdx/math/RandomXS128.java")
    parser.add_argument("--output", type=Path, default=ROOT / "tests/fixtures/java_rng.tsv")
    args = parser.parse_args()
    def executable(name):
        return str(args.java_home / "bin" / name) if args.java_home else name
    harness = ROOT / "tools/RngOracle.java"
    # Fail on javac errors; never silently patch the user's reference source.
    with tempfile.TemporaryDirectory(prefix="rusted-spire-rng-") as directory:
        subprocess.run([executable("javac"), "-d", directory, str(args.source), str(harness)], check=True)
        result = subprocess.run([executable("java"), "-cp", directory, "RngOracle"],
                                check=True, capture_output=True, text=True)
    # Keep numeric rows only: JVM runtime warnings must not become fixture data.
    rows = [line for line in result.stdout.splitlines() if len(line.split("\t")) == 6]
    if len(rows) != 1283:
        raise RuntimeError(f"unexpected oracle output: {len(rows)} rows")
    args.output.parent.mkdir(parents=True, exist_ok=True)
    data = "# seed\top\targ1\targ2\texpected\tcounter\n" + "\n".join(rows) + "\n"
    args.output.write_text(data)
    version = subprocess.run([executable("java"), "-version"], check=True, capture_output=True, text=True)
    sources = [args.source, harness,
               ROOT / "decompiled/sources/com/megacrit/cardcrawl/random/Random.java",
               ROOT / "decompiled/sources/com/megacrit/cardcrawl/cards/CardGroup.java",
               ROOT / "decompiled/sources/com/megacrit/cardcrawl/cards/Soul.java",
               ROOT / "decompiled/sources/com/megacrit/cardcrawl/actions/common/EmptyDeckShuffleAction.java",
               ROOT / "decompiled/sources/com/megacrit/cardcrawl/dungeons/AbstractDungeon.java"]
    metadata = {
        "scope": "Executed local RandomXS128 and JDK Collections.shuffle; game wrapper call mapping is static.",
        "java_version": (version.stderr + version.stdout).strip(),
        "fixture_sha256": hashlib.sha256(data.encode()).hexdigest(),
        "sources": {str(p.relative_to(ROOT) if p.is_relative_to(ROOT) else p.name):
                    hashlib.sha256(p.read_bytes()).hexdigest() for p in sources},
        "rows": len(rows),
    }
    args.output.with_suffix(".json").write_text(json.dumps(metadata, indent=2) + "\n")
    print(f"Wrote {len(rows)} Java-generated cases to {args.output}")


if __name__ == "__main__":
    main()
