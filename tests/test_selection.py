import importlib.util
import random
import unittest

import rusted_spire as rs


def play_action(obs, mask, name):
    ordinal = rs.CARD_NAMES.index(name) + 1
    for slot in range(rs.MAX_HAND):
        if round(obs[rs.PLAYER_FEATURES + slot * rs.HAND_FEATURES] * rs.CARD_COUNT) == ordinal:
            for action in range(slot * rs.TARGETS_PER_CARD, (slot + 1) * rs.TARGETS_PER_CARD):
                if mask[action]:
                    return action
    raise AssertionError(f"No playable {name}")


class SelectionTests(unittest.TestCase):
    def test_each_choice_kind_and_illegal_actions_preserve_pending_state(self):
        for name, kind, deck in [
            ("Armaments", 0, ["Armaments", "Strike", "Defend"]),
            ("TrueGrit", 1, ["TrueGrit+", "Strike", "Defend"]),
            ("Warcry", 2, ["Warcry", "Strike", "Defend"]),
            ("Headbutt", 3, ["Headbutt", "Defend", "Defend"]),
        ]:
            with self.subTest(name=name):
                env, reference = rs.SlayEnv(deck=deck), rs.SlayEnv(deck=deck)
                obs, mask = env.reset(42)
                reference.reset(42)
                if name == "Headbutt":
                    for _ in range(2):
                        action = play_action(obs, mask, "Defend")
                        result = env.step(action)
                        self.assertEqual(result, reference.step(action))
                        obs, mask = result[:2]
                action = play_action(obs, mask, name)
                result = env.step(action)
                self.assertEqual(result, reference.step(action))
                obs, mask, reward, done = result
                self.assertEqual(obs[rs.SELECTION_OFFSET + kind], 1)
                self.assertEqual(reward, 0)
                self.assertFalse(done)
                self.assertFalse(any(mask[:rs.SELECT_CARD_ACTION]))
                self.assertEqual(sum(mask), 2)
                self.assertEqual(obs[rs.CHOICE_OFFSET], 1)
                for bad in (action, rs.END_TURN_ACTION, rs.SELECT_CARD_ACTION + 2,
                            rs.PREVIOUS_PAGE_ACTION, rs.NEXT_PAGE_ACTION, rs.ACTION_SIZE):
                    with self.assertRaises(ValueError):
                        env.step(bad)
                result = env.step(rs.SELECT_CARD_ACTION + 1)
                self.assertEqual(result, reference.step(rs.SELECT_CARD_ACTION + 1))
                self.assertTrue(all(x == 0 for x in result[0][rs.SELECTION_OFFSET:]))
                self.assertTrue(result[1][rs.END_TURN_ACTION])

    def test_reset_clears_pending_choices_and_in_combat_upgrades(self):
        for card in ("Armaments", "Armaments+"):
            env = rs.SlayEnv(deck=[card, "BodySlam", "Defend"])
            initial = env.reset(17)
            action = play_action(*initial, "Armaments")
            env.step(action)
            self.assertEqual(env.reset(17), initial)
            if card == "Armaments":
                env.step(action)
                env.step(rs.SELECT_CARD_ACTION)
                self.assertEqual(env.reset(17), initial)

    def test_python_can_select_from_second_discard_page(self):
        env = rs.SlayEnv(enemy="Cultist", ascension=0, deck=["Headbutt"] * 25)
        env.reset(42)
        for _ in range(3):
            obs, mask, _, done = env.step(rs.END_TURN_ACTION)
            self.assertFalse(done)
        obs, mask, _, done = env.step(play_action(obs, mask, "Headbutt"))
        self.assertFalse(done)
        self.assertEqual(obs[rs.SELECTION_OFFSET + 5], 1.5)
        self.assertTrue(mask[rs.NEXT_PAGE_ACTION])
        before = obs[:rs.SELECTION_OFFSET]
        obs, mask, reward, done = env.step(rs.NEXT_PAGE_ACTION)
        self.assertEqual(obs[:rs.SELECTION_OFFSET], before)
        self.assertEqual(sum(mask), 6)  # five candidates plus previous page
        self.assertEqual(reward, 0)
        self.assertFalse(done)
        self.assertFalse(mask[rs.NEXT_PAGE_ACTION])
        with self.assertRaises(ValueError):
            env.step(rs.SELECT_CARD_ACTION + 5)
        obs, mask, _, _ = env.step(rs.SELECT_CARD_ACTION + 4)
        self.assertTrue(mask[rs.END_TURN_ACTION])
        self.assertTrue(all(v == 0 for v in obs[rs.SELECTION_OFFSET:]))

    @unittest.skipUnless(importlib.util.find_spec("gymnasium"), "requires train dependencies")
    def test_seeded_selection_episodes_are_reproducible_and_fit_gym_spaces(self):
        from spire_env import SpireEnv
        deck = ["Armaments", "TrueGrit+", "Warcry+", "Headbutt", "Strike", "Defend", "BodySlam", "ShrugItOff"]
        choices = 0
        for enemy in ("Cultist", "GremlinNob", "TheGuardian"):
            env = SpireEnv(enemy=enemy, deck=deck, ascension=0)
            reference = SpireEnv(enemy=enemy, deck=deck, ascension=0)
            for seed in range(10):
                policy = random.Random(seed)
                obs, _ = env.reset(seed=seed)
                other, _ = reference.reset(seed=seed)
                self.assertEqual(obs.tolist(), other.tolist())
                for _ in range(1000):
                    self.assertTrue(env.observation_space.contains(obs))
                    mask = env.action_masks()
                    choices += int(any(obs[rs.SELECTION_OFFSET:rs.SELECTION_OFFSET + 4]))
                    action = policy.choice([i for i, legal in enumerate(mask) if legal])
                    obs, reward, done, truncated, info = env.step(action)
                    other, r2, d2, t2, i2 = reference.step(action)
                    self.assertEqual(obs.tolist(), other.tolist())
                    self.assertEqual((reward, done, truncated, info), (r2, d2, t2, i2))
                    self.assertFalse(truncated)
                    if done:
                        break
                else:
                    self.fail("episode did not finish")
            env.close()
            reference.close()
        self.assertGreater(choices, 20)


if __name__ == "__main__":
    unittest.main()
