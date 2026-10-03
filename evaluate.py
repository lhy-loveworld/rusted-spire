"""Evaluate random or trained policies on an explicit, repeatable seed range."""

import argparse
import contextlib
import json
import sys

import numpy as np

import rusted_spire
from spire_env import SpireEnv


def evaluate(env, episodes=100, start_seed=100_000, policy_seed=1234, model=None):
    if episodes < 1:
        raise ValueError("episodes must be positive")
    rng = np.random.default_rng(policy_seed)
    wins = truncations = 0
    hp_on_victory = []
    rewards = []
    lengths = []
    for seed in range(start_seed, start_seed + episodes):
        obs, _ = env.reset(seed=seed)
        reward_sum = 0.0
        steps = 0
        while True:
            mask = env.action_masks()
            if model is None:
                action = int(rng.choice(np.flatnonzero(mask)))
            else:
                action, _ = model.predict(obs, action_masks=mask, deterministic=True)
                action = int(action)
            obs, reward, terminated, truncated, _ = env.step(action)
            reward_sum += reward
            steps += 1
            if terminated or truncated:
                truncations += int(truncated)
                if terminated and reward > 0:
                    wins += 1
                    hp_on_victory.append(float(obs[0] * rusted_spire.MAX_HP))
                rewards.append(reward_sum)
                lengths.append(steps)
                break
    return {
        "interface_version": rusted_spire.INTERFACE_VERSION,
        "episodes": episodes, "start_seed": start_seed, "policy_seed": policy_seed,
        "wins": wins, "win_rate": wins / episodes, "truncations": truncations,
        "mean_reward": float(np.mean(rewards)), "mean_steps": float(np.mean(lengths)),
        "mean_hp_on_victory": float(np.mean(hp_on_victory)) if hp_on_victory else None,
    }


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--model", help="optional PPO checkpoint; omit for random baseline")
    parser.add_argument("--episodes", type=int, default=100)
    parser.add_argument("--start-seed", type=int, default=100_000)
    parser.add_argument("--policy-seed", type=int, default=1234)
    parser.add_argument("--enemies", nargs="+", default=["Cultist"])
    parser.add_argument("--ascension", type=int, default=7)
    args = parser.parse_args()
    env = SpireEnv(enemies=args.enemies, ascension=args.ascension)
    try:
        model = None
        if args.model:
            import torch
            from sb3_contrib import MaskablePPO
            torch.set_num_threads(1)
            # Supplying env makes SB3 reject old observation/action dimensions.
            with contextlib.redirect_stdout(sys.stderr):
                model = MaskablePPO.load(args.model, env=env, device="cpu")
        result = evaluate(env, args.episodes, args.start_seed, args.policy_seed, model)
        result.update(enemies=args.enemies, ascension=args.ascension,
                      policy=args.model or "random")
        print(json.dumps(result, indent=2))
    finally:
        env.close()


if __name__ == "__main__":
    main()
