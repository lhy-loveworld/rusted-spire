"""Train a masked PPO policy. Defaults to Cultist at ascension 7."""

import argparse
import json
import time
import subprocess
from importlib.metadata import version
from pathlib import Path

import torch
from sb3_contrib import MaskablePPO
from sb3_contrib.common.maskable.callbacks import MaskableEvalCallback
from stable_baselines3.common.env_util import make_vec_env
from stable_baselines3.common.vec_env import SubprocVecEnv

import rusted_spire
from spire_env import SpireEnv


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--timesteps", type=float, default=1_000_000)
    parser.add_argument("--n-envs", type=int, default=8)
    parser.add_argument("--save-path", default=f"models/ppo_spire_v{rusted_spire.INTERFACE_VERSION}")
    parser.add_argument("--seed", type=int, default=42)
    parser.add_argument("--enemies", nargs="+", default=["Cultist"])
    parser.add_argument("--ascension", type=int, default=7)
    parser.add_argument("--n-steps", type=int, default=512)
    parser.add_argument("--eval-freq", type=int, default=20_000, help="environment timesteps between evaluations")
    parser.add_argument("--eval-episodes", type=int, default=100)
    args = parser.parse_args()
    if min(args.n_envs, args.n_steps, args.timesteps, args.eval_freq, args.eval_episodes) <= 0:
        parser.error("training sizes and evaluation intervals must be positive")
    rollout_size = args.n_envs * args.n_steps
    if rollout_size < 2:
        parser.error("n-envs * n-steps must be at least 2")

    # Small MLP policies are faster without oversubscribing CPU threads.
    torch.set_num_threads(1)
    save_path = Path(args.save_path)
    save_path.mkdir(parents=True, exist_ok=True)
    metadata = {
        "interface_version": rusted_spire.INTERFACE_VERSION,
        "obs_size": rusted_spire.OBS_SIZE,
        "action_size": rusted_spire.ACTION_SIZE,
        "training": vars(args),
        "revision": subprocess.check_output(["git", "rev-parse", "HEAD"], text=True).strip(),
        "tracked_changes": bool(subprocess.check_output(["git", "diff", "HEAD", "--name-only"], text=True).strip()),
        "packages": {name: version(name) for name in ("torch", "stable-baselines3", "sb3-contrib", "gymnasium", "numpy")},
    }
    (save_path / "interface.json").write_text(json.dumps(metadata, indent=2) + "\n")
    kwargs = {"enemies": args.enemies, "ascension": args.ascension}
    vec_env = make_vec_env(SpireEnv, n_envs=args.n_envs, seed=args.seed,
                          env_kwargs=kwargs, vec_env_cls=SubprocVecEnv)
    eval_env = None
    try:
        eval_env = make_vec_env(SpireEnv, n_envs=1, seed=args.seed + 1_000_000,
                               env_kwargs=kwargs, vec_env_cls=SubprocVecEnv)
        model = MaskablePPO(
            "MlpPolicy", vec_env, verbose=1, learning_rate=3e-4,
            n_steps=args.n_steps, batch_size=min(256, rollout_size),
            n_epochs=10, gamma=0.99, tensorboard_log="runs/",
            seed=args.seed, device="cpu",
        )
        callback = MaskableEvalCallback(
            eval_env, best_model_save_path=str(save_path), log_path=str(save_path / "evaluation"),
            eval_freq=max(args.eval_freq // args.n_envs, 1),
            n_eval_episodes=args.eval_episodes, deterministic=True, verbose=1,
        )
        start = time.time()
        model.learn(total_timesteps=int(args.timesteps), callback=callback)
        model.save(str(save_path / "final"))
        print(f"Done in {time.time() - start:.1f}s; model saved to {save_path / 'final'}")
    finally:
        vec_env.close()
        if eval_env is not None:
            eval_env.close()


if __name__ == "__main__":
    main()
