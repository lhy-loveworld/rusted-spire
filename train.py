"""
Phase 9: MaskablePPO on Ironclad vs Cultist (ascension 7 by default).

Usage:
    python train.py                  # train from scratch
    python train.py --timesteps 2e6  # custom budget
"""

import argparse
import random
import time

import gymnasium as gym
import numpy as np
from gymnasium import spaces
from sb3_contrib import MaskablePPO
from sb3_contrib.common.maskable.callbacks import MaskableEvalCallback
from sb3_contrib.common.maskable.utils import get_action_masks

import rusted_spire


class SpireEnv(gym.Env):
    """Gymnasium wrapper around the Rust SlayEnv."""

    metadata = {"render_modes": []}

    def __init__(self):
        super().__init__()
        self._env = rusted_spire.SlayEnv()
        self._action_mask = np.ones(rusted_spire.ACTION_SIZE, dtype=bool)

        self.observation_space = spaces.Box(
            low=-1.0,
            high=2.0,
            shape=(rusted_spire.OBS_SIZE,),
            dtype=np.float32,
        )
        self.action_space = spaces.Discrete(rusted_spire.ACTION_SIZE)

    def reset(self, *, seed=None, options=None):
        super().reset(seed=seed)
        s = seed if seed is not None else random.randint(0, 2**32 - 1)
        obs, mask = self._env.reset(s)
        self._action_mask = np.array(mask, dtype=bool)
        return np.array(obs, dtype=np.float32), {}

    def step(self, action: int):
        obs, mask, reward, done = self._env.step(int(action))
        self._action_mask = np.array(mask, dtype=bool)
        return np.array(obs, dtype=np.float32), reward, done, False, {}

    def action_masks(self) -> np.ndarray:
        return self._action_mask


def make_env():
    def _init():
        return SpireEnv()
    return _init


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--timesteps", type=float, default=1_000_000)
    parser.add_argument("--n-envs", type=int, default=8)
    parser.add_argument("--save-path", type=str, default="models/ppo_spire")
    args = parser.parse_args()

    from stable_baselines3.common.env_util import make_vec_env
    from stable_baselines3.common.vec_env import SubprocVecEnv

    print(f"Obs size: {rusted_spire.OBS_SIZE}  Action size: {rusted_spire.ACTION_SIZE}")
    print(f"Training for {int(args.timesteps):,} timesteps across {args.n_envs} envs\n")

    vec_env = make_vec_env(
        SpireEnv,
        n_envs=args.n_envs,
        vec_env_cls=SubprocVecEnv,
    )
    eval_env = make_vec_env(SpireEnv, n_envs=4, vec_env_cls=SubprocVecEnv)

    model = MaskablePPO(
        "MlpPolicy",
        vec_env,
        verbose=1,
        learning_rate=3e-4,
        n_steps=512,
        batch_size=256,
        n_epochs=10,
        gamma=0.99,
        tensorboard_log="runs/",
    )

    eval_callback = MaskableEvalCallback(
        eval_env,
        best_model_save_path=args.save_path,
        log_path="logs/",
        eval_freq=20_000,
        n_eval_episodes=200,
        deterministic=True,
        verbose=1,
    )

    t0 = time.time()
    model.learn(total_timesteps=int(args.timesteps), callback=eval_callback)
    elapsed = time.time() - t0

    import os
    os.makedirs(args.save_path, exist_ok=True)
    model.save(f"{args.save_path}/final")
    print(f"\nDone in {elapsed:.1f}s — model saved to {args.save_path}/final")


if __name__ == "__main__":
    main()
