"""Gymnasium adapter shared by training, evaluation, and notebooks."""

import gymnasium as gym
import numpy as np
from gymnasium import spaces

import rusted_spire


class SpireEnv(gym.Env):
    metadata = {"render_modes": []}

    def __init__(self, enemy="Cultist", *, enemies=None, ascension=7, max_steps=1000, deck=None):
        super().__init__()
        if max_steps < 1:
            raise ValueError("max_steps must be positive")
        self._env = rusted_spire.SlayEnv(enemy=enemy, enemies=enemies, ascension=ascension, deck=deck)
        self._max_steps = max_steps
        self._steps = 0
        self._needs_reset = True
        self._action_mask = np.zeros(rusted_spire.ACTION_SIZE, dtype=bool)
        # Normalization scales are units, not hard bounds: Strength, status-card
        # cost and growing piles can exceed the old [-1, 2] bounds.
        self.observation_space = spaces.Box(
            low=-np.inf, high=np.inf, shape=(rusted_spire.OBS_SIZE,), dtype=np.float32
        )
        self.action_space = spaces.Discrete(rusted_spire.ACTION_SIZE)

    def reset(self, *, seed=None, options=None):
        super().reset(seed=seed)
        combat_seed = int(self.np_random.integers(0, 2**32))
        obs, mask = self._env.reset(combat_seed, hp=(options or {}).get("hp"))
        self._action_mask = np.asarray(mask, dtype=bool)
        self._steps = 0
        self._needs_reset = False
        return np.asarray(obs, dtype=np.float32), {"combat_seed": combat_seed}

    def step(self, action):
        if self._needs_reset:
            raise RuntimeError("call reset() before stepping a new episode")
        if not self.action_space.contains(action):
            raise ValueError("action must be an integer in the action space")
        obs, mask, reward, terminated = self._env.step(int(action))
        self._steps += 1
        truncated = not terminated and self._steps >= self._max_steps
        self._needs_reset = terminated or truncated
        self._action_mask = np.asarray(mask, dtype=bool)
        info = {}
        if terminated:
            info["is_success"] = reward > 0
        if truncated:
            self._action_mask[:] = False
        return np.asarray(obs, dtype=np.float32), reward, terminated, truncated, info

    def action_masks(self):
        return self._action_mask.copy()

    @property
    def deck(self):
        return self._env.deck
