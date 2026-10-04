import unittest
import importlib.util
import json
from pathlib import Path

TRAINING_AVAILABLE = all(importlib.util.find_spec(name) is not None
                         for name in ("numpy", "gymnasium"))
if TRAINING_AVAILABLE:
    import numpy as np
    from spire_env import SpireEnv

import rusted_spire


@unittest.skipUnless(TRAINING_AVAILABLE, "install the train extra for Gymnasium tests")
class GymContractTests(unittest.TestCase):
    def test_seed_reproduces_following_unseeded_resets(self):
        a, b = SpireEnv(), SpireEnv()
        for seed in (42, None, None, 5, None):
            obs_a, info_a = a.reset(seed=seed)
            obs_b, info_b = b.reset(seed=seed)
            np.testing.assert_array_equal(obs_a, obs_b)
            self.assertEqual(info_a, info_b)

    def test_truncation_is_distinct_from_defeat(self):
        env = SpireEnv(max_steps=1)
        env.reset(seed=42)
        _, reward, terminated, truncated, _ = env.step(rusted_spire.END_TURN_ACTION)
        self.assertFalse(terminated)
        self.assertTrue(truncated)
        self.assertEqual(reward, 0)
        self.assertFalse(env.action_masks().any())
        with self.assertRaises(RuntimeError):
            env.step(rusted_spire.END_TURN_ACTION)

        env = SpireEnv(enemy="JawWorm", max_steps=1)
        env.reset(seed=42, options={"hp": 1})
        _, reward, terminated, truncated, info = env.step(rusted_spire.END_TURN_ACTION)
        self.assertTrue(terminated)
        self.assertFalse(truncated)
        self.assertEqual(reward, -1)
        self.assertFalse(info["is_success"])

    def test_observations_fit_space_through_multi_enemy_episodes(self):
        env = SpireEnv(enemies=["LouseNormal", "LouseDefensive"])
        rng = np.random.default_rng(42)
        for seed in range(20):
            obs, _ = env.reset(seed=seed)
            self.assertTrue(env.observation_space.contains(obs))
            for _ in range(1000):
                action = int(rng.choice(np.flatnonzero(env.action_masks())))
                obs, _, done, truncated, _ = env.step(action)
                self.assertTrue(env.observation_space.contains(obs))
                self.assertTrue(np.isfinite(obs).all())
                if done or truncated:
                    break
            else:
                self.fail("episode did not finish")

    def test_masks_cannot_be_mutated_by_caller(self):
        env = SpireEnv()
        env.reset(seed=42)
        returned = env.action_masks()
        returned[:] = False
        self.assertTrue(env.action_masks().any())

    def test_evaluation_is_repeatable(self):
        from evaluate import evaluate
        env = SpireEnv()
        self.assertEqual(evaluate(env, episodes=10), evaluate(env, episodes=10))

    def test_evaluation_records_preserve_truncations_and_seed_provenance(self):
        from evaluate import evaluate
        class EndTurnPolicy:
            def predict(self, obs, **kwargs):
                return rusted_spire.END_TURN_ACTION, None
        env = SpireEnv(max_steps=1)
        result = evaluate(env, episodes=5, model=EndTurnPolicy(), include_episodes=True)
        self.assertEqual(result["wins"], 0)
        self.assertEqual(result["truncations"], 5)
        self.assertEqual(result["mean_steps"], 1)
        self.assertEqual(result["mean_reward"], 0)
        self.assertGreater(result["win_rate_wilson95"][1], 0)
        for record, seed in zip(result["episode_results"], range(100000, 100005)):
            _, info = env.reset(seed=seed)
            self.assertEqual(record["environment_seed"], seed)
            self.assertEqual(record["combat_seed"], info["combat_seed"])
            self.assertTrue(record["truncated"])
            self.assertFalse(record["win"])
            self.assertGreater(record["enemy_hp_remaining"], 0)

    def test_notebook_cells_compile_and_chain_uses_shared_wrapper(self):
        import gymnasium as gym
        notebook = json.loads((Path(__file__).parents[1] / "experiments.ipynb").read_text())
        namespace = {"gym": gym, "np": np, "rusted_spire": rusted_spire, "SpireEnv": SpireEnv}
        for cell in notebook["cells"]:
            if cell["cell_type"] != "code":
                continue
            source = cell["source"]
            source = "".join(source) if isinstance(source, list) else source
            compile(source, "experiments.ipynb", "exec")
            if source.startswith("class MultiCombatEnv"):
                exec(source, namespace)
        env = namespace["MultiCombatEnv"](enemies=["Cultist"] * 2, max_steps=1)
        obs, _ = env.reset(seed=42)
        self.assertTrue(env.observation_space.contains(obs))
        _, _, terminated, truncated, _ = env.step(rusted_spire.END_TURN_ACTION)
        self.assertFalse(terminated)
        self.assertTrue(truncated)
        self.assertFalse(env.action_masks().any())
        with self.assertRaises(RuntimeError):
            env.step(rusted_spire.END_TURN_ACTION)
        env.close()


if __name__ == "__main__":
    unittest.main()
