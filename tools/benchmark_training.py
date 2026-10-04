"""Compare independently trained final checkpoints with uniform legal-action baselines.

Run with .venv/bin/python. Checkpoints/logs remain under the ignored output folder.
The same held-out environment seeds are used for every policy. Test results do
not select checkpoints or tune training; train.py's separate validation stream
is used only for its periodic evaluations.
"""
import argparse
import hashlib
import json
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT))

import torch
from sb3_contrib import MaskablePPO

from evaluate import evaluate
from spire_env import SpireEnv


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--encounters", nargs="+", default=["Lagavulin", "TheGuardian"])
    parser.add_argument("--train-seeds", nargs="+", type=int, default=[11, 22, 33])
    parser.add_argument("--timesteps", type=int, default=65536)
    parser.add_argument("--episodes", type=int, default=300)
    parser.add_argument("--start-seed", type=int, default=200000)
    parser.add_argument("--ascension", type=int, default=0)
    parser.add_argument("--output", type=Path, default=ROOT / "models/benchmark_v4")
    args = parser.parse_args()
    args.output = args.output.resolve()
    if args.output.exists():
        parser.error("output already exists; choose a new directory to preserve prior runs")
    if args.timesteps < 1 or args.episodes < 1:
        parser.error("timesteps and episodes must be positive")
    if subprocess.check_output(["git", "diff", "HEAD", "--name-only"], cwd=ROOT, text=True).strip():
        parser.error("commit tracked changes first so every result has a reproducible revision")
    args.output.mkdir(parents=True)
    torch.set_num_threads(1)
    report = {
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True).strip(),
        "protocol": dict(vars(args), output=str(args.output)),
        "checkpoint_selection": "final, never selected using held-out results",
        "results": [],
    }

    def record(encounter, policy, result, **extra):
        entry = dict(result, encounter=encounter, policy=policy, **extra)
        report["results"].append(entry)
        (args.output / "results.json").write_text(json.dumps(report, indent=2) + "\n")
        print(json.dumps({k: v for k, v in entry.items() if k != "episode_results"}), flush=True)

    for encounter in args.encounters:
        env = SpireEnv(enemy=encounter, ascension=args.ascension)
        try:
            for seed in args.train_seeds:
                baseline = evaluate(env, args.episodes, args.start_seed, seed, include_episodes=True)
                record(encounter, "random", baseline)
                run_dir = args.output / f"{encounter}_{seed}"
                command = [sys.executable, str(ROOT / "train.py"),
                    "--timesteps", str(args.timesteps), "--seed", str(seed),
                    "--enemies", encounter, "--ascension", str(args.ascension),
                    "--n-envs", "2", "--n-steps", "128",
                    "--eval-freq", str(args.timesteps), "--eval-episodes", "20",
                    "--save-path", str(run_dir)]
                print(f"Training {encounter}, seed {seed}, {args.timesteps} steps", flush=True)
                with (args.output / f"{encounter}_{seed}.log").open("w") as log:
                    subprocess.run(command, cwd=ROOT, stdout=log, stderr=subprocess.STDOUT, check=True)
                checkpoint = run_dir / "final.zip"
                model = MaskablePPO.load(checkpoint, env=env, device="cpu")
                result = evaluate(env, args.episodes, args.start_seed, seed, model, include_episodes=True)
                record(encounter, "ppo_final", result, training_seed=seed,
                       checkpoint_sha256=hashlib.sha256(checkpoint.read_bytes()).hexdigest(),
                       training_metadata=json.loads((run_dir / "interface.json").read_text()))
        finally:
            env.close()


if __name__ == "__main__":
    main()
