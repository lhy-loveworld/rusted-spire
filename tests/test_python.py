"""Behavioral checks for the installed extension; no training packages required."""

import math
import random
import unittest

import rusted_spire


class SlayEnvTests(unittest.TestCase):
    def test_versioned_interface_and_explicit_targeting(self):
        self.assertEqual(rusted_spire.INTERFACE_VERSION, 5)
        self.assertEqual(rusted_spire.ACTION_SIZE, 73)
        self.assertEqual(rusted_spire.OBS_SIZE, 242)
        env = rusted_spire.SlayEnv(enemies=["JawWorm", "JawWorm"])
        before, mask = env.reset(42)
        action = next(i for i, legal in enumerate(mask)
                      if legal and i % rusted_spire.TARGETS_PER_CARD == 1)
        after, _, _, done = env.step(action)
        offset = rusted_spire.ENEMY_OFFSET
        self.assertFalse(done)
        self.assertEqual(after[offset + 2], before[offset + 2])
        self.assertLess(after[offset + rusted_spire.ENEMY_FEATURES + 2],
                        before[offset + rusted_spire.ENEMY_FEATURES + 2])

    def test_spawn_overflow_rejects_action_without_mutation(self):
        enemies = ["AcidSlimeLarge"] + ["ShieldGremlin"] * 4
        env = rusted_spire.SlayEnv(enemies=enemies, ascension=0)
        reference = rusted_spire.SlayEnv(enemies=enemies, ascension=0)
        obs, mask = env.reset(2)
        reference.reset(2)
        for _ in range(100):
            if obs[rusted_spire.ENEMY_OFFSET + 5] == 1.0:  # pending Split
                for _ in range(2):
                    with self.assertRaisesRegex(ValueError, "capacity"):
                        env.step(rusted_spire.END_TURN_ACTION)
                # Seed 2 reaches the split with energy and a Defend left.
                # Compare a successful transition to an untouched reference,
                # including powers, hand, HP, mask and RNG-dependent state.
                safe = next(i for i, legal in enumerate(mask)
                            if legal and i != rusted_spire.END_TURN_ACTION)
                self.assertEqual(env.step(safe), reference.step(safe))
                break
            attacks = [i for i, legal in enumerate(mask)
                       if legal and i != rusted_spire.END_TURN_ACTION
                       and i % rusted_spire.TARGETS_PER_CARD == 0]
            action = attacks[0] if attacks else rusted_spire.END_TURN_ACTION
            result = env.step(action)
            self.assertEqual(result, reference.step(action))
            obs, mask, _, done = result
            self.assertFalse(done)
        else:
            self.fail("expected a split to exceed observation capacity")

    def test_reset_is_reproducible(self):
        env = rusted_spire.SlayEnv()
        initial = env.reset(42)
        env.step(rusted_spire.END_TURN_ACTION)
        self.assertEqual(env.reset(42), initial)

    def test_step_requires_reset(self):
        with self.assertRaises(RuntimeError):
            rusted_spire.SlayEnv().step(0)

    def test_invalid_actions_do_not_mutate_state(self):
        env = rusted_spire.SlayEnv(enemy="JawWorm")
        reference = rusted_spire.SlayEnv(enemy="JawWorm")
        _, mask = env.reset(42)
        reference.reset(42)
        invalid = [i for i, legal in enumerate(mask) if not legal]
        invalid.extend([rusted_spire.ACTION_SIZE, 1000])
        for action in invalid:
            with self.subTest(action=action), self.assertRaises(ValueError):
                env.step(action)
        end_turn = rusted_spire.END_TURN_ACTION
        self.assertEqual(env.step(end_turn), reference.step(end_turn))

    def test_unaffordable_cards_are_rejected(self):
        env = rusted_spire.SlayEnv()
        _, mask = env.reset(42)
        while any(mask[:rusted_spire.END_TURN_ACTION]):
            _, mask, _, done = env.step(next(i for i in range(rusted_spire.END_TURN_ACTION) if mask[i]))
            self.assertFalse(done)
        # The starter hand has five cards and spending three energy cannot
        # consume all of them; slot zero is occupied but unaffordable now.
        with self.assertRaises(ValueError):
            env.step(0)

    def test_terminal_requires_reset(self):
        env = rusted_spire.SlayEnv(enemy="JawWorm")
        env.reset(42, hp=1)
        _, mask, reward, done = env.step(rusted_spire.END_TURN_ACTION)
        self.assertTrue(done)
        self.assertEqual(reward, -1.0)
        self.assertFalse(any(mask))
        with self.assertRaises(RuntimeError):
            env.step(0)
        self.assertTrue(any(env.reset(42)[1]))

    def test_enemy_count_matches_observation_capacity(self):
        for enemies in ([], ["JawWorm"] * 6):
            with self.subTest(enemies=enemies), self.assertRaises(ValueError):
                rusted_spire.SlayEnv(enemies=enemies)

    def test_random_policy_completes_100_episodes(self):
        policy = random.Random(1234)
        encounters = (["JawWorm"], ["Cultist"], ["LouseNormal", "LouseDefensive"],
                      ["Sentry"] * 3, ["SlimeBoss"], ["AcidSlimeLarge", "SpikeSlimeLarge"],
                      ["Lagavulin"], ["TheGuardian"])
        for seed in range(100):
            env = rusted_spire.SlayEnv(enemies=encounters[seed % len(encounters)])
            obs, mask = env.reset(seed)
            for _ in range(1000):
                self.assertEqual(len(obs), rusted_spire.OBS_SIZE)
                self.assertTrue(all(math.isfinite(value) for value in obs))
                self.assertEqual(len(mask), rusted_spire.ACTION_SIZE)
                legal = [i for i, allowed in enumerate(mask) if allowed]
                self.assertTrue(legal)
                obs, mask, reward, done = env.step(policy.choice(legal))
                if done:
                    self.assertFalse(any(mask))
                    self.assertTrue(reward == -1.0 or 1.0 <= reward <= 2.0)
                    break
            else:
                self.fail(f"episode {seed} exceeded the step budget")


if __name__ == "__main__":
    unittest.main()
