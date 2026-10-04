"""Evaluate random or trained policies on an explicit, repeatable seed range."""

import argparse
import contextlib
import json
import math
import sys

import numpy as np

import rusted_spire
from spire_env import SpireEnv
from deck_config import STARTER, add_deck_arguments, resolve_deck, model_deck


def evaluate(env, episodes=100, start_seed=100_000, policy_seed=1234, model=None, *, include_episodes=False):
    if episodes < 1:
        raise ValueError("episodes must be positive")
    rng = np.random.default_rng(policy_seed)
    wins = truncations = 0
    hp_on_victory = []
    rewards = []
    lengths = []
    records = []
    for seed in range(start_seed, start_seed + episodes):
        obs, reset_info = env.reset(seed=seed)
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
                remaining_hp = sum(
                    float(obs[i + 2] * obs[i + 3] * 300)
                    for i in range(rusted_spire.ENEMY_OFFSET, rusted_spire.OBS_SIZE,
                                   rusted_spire.ENEMY_FEATURES)
                )
                records.append({
                    "environment_seed": seed, "combat_seed": reset_info["combat_seed"],
                    "win": bool(terminated and reward > 0), "truncated": bool(truncated),
                    "reward": reward_sum, "steps": steps,
                    "player_hp": float(obs[0] * rusted_spire.MAX_HP),
                    "enemy_hp_remaining": remaining_hp,
                })
                break
    # Wilson interval describes evaluation-seed uncertainty for this one policy,
    # not variation across independent training runs.
    rate, z = wins / episodes, 1.959963984540054
    denominator = 1 + z * z / episodes
    center = (rate + z * z / (2 * episodes)) / denominator
    radius = z * math.sqrt(rate * (1 - rate) / episodes + z * z / (4 * episodes * episodes)) / denominator
    result = {
        "interface_version": rusted_spire.INTERFACE_VERSION,
        "deck": env.deck,
        "episodes": episodes, "start_seed": start_seed, "policy_seed": policy_seed,
        "wins": wins, "win_rate": wins / episodes, "truncations": truncations,
        "win_rate_wilson95": [max(0.0, center - radius), min(1.0, center + radius)],
        "mean_reward": float(np.mean(rewards)), "mean_steps": float(np.mean(lengths)),
        "mean_hp_on_victory": float(np.mean(hp_on_victory)) if hp_on_victory else None,
        "mean_enemy_hp_remaining": float(np.mean([r["enemy_hp_remaining"] for r in records])),
    }
    if include_episodes:
        result["episode_results"] = records
    return result


def main():
    parser = argparse.ArgumentParser()
    add_deck_arguments(parser, default_preset=None)
    parser.add_argument("--model", help="optional PPO checkpoint; omit for random baseline")
    parser.add_argument("--episodes", type=int, default=100)
    parser.add_argument("--start-seed", type=int, default=100_000)
    parser.add_argument("--policy-seed", type=int, default=1234)
    parser.add_argument("--enemies", nargs="+", default=["Cultist"])
    parser.add_argument("--ascension", type=int, default=7)
    args = parser.parse_args()
    try:
        deck = resolve_deck(args)
        if deck is None:
            deck = model_deck(args.model) if args.model else list(STARTER)
    except (ValueError, OSError) as error:
        parser.error(str(error))
    env = SpireEnv(enemies=args.enemies, ascension=args.ascension, deck=deck)
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
