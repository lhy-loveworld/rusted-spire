import argparse
import importlib.util
import json
from pathlib import Path
import random
import tempfile
import unittest

import rusted_spire as rs
from deck_config import PRESETS, STARTER, add_deck_arguments, resolve_deck, model_deck


class DeckTests(unittest.TestCase):
    def test_default_matches_explicit_starter(self):
        for seed in (0, 1, 42):
            a, b = rs.SlayEnv(), rs.SlayEnv(deck=list(STARTER))
            self.assertEqual(a.reset(seed), b.reset(seed))
            self.assertEqual(a.step(rs.END_TURN_ACTION), b.step(rs.END_TURN_ACTION))

    def test_deck_and_reset_do_not_share_mutable_card_instances(self):
        cards = ["Inflame+"]
        env = rs.SlayEnv(deck=cards)
        cards[0] = "Strike"
        initial = env.reset(42)
        env.deck.append("Defend")
        after, _, _, _ = env.step(rs.UNTARGETED_SLOT)
        self.assertAlmostEqual(after[48], 0.3)
        self.assertEqual(after[5], 0)  # powers do not enter exhaust
        self.assertEqual(env.deck, ["Inflame+"])
        self.assertEqual(env.reset(42), initial)

    def test_upgrade_cost_stats_and_duplicates_reach_python(self):
        env = rs.SlayEnv(deck=["Defend+"] * 5)
        obs, _ = env.reset(0)
        self.assertEqual(len(env.deck), 5)
        self.assertTrue(all(obs[8 + h * 4 + 2] == 1 for h in range(5)))
        obs, _, _, _ = env.step(rs.UNTARGETED_SLOT)
        self.assertAlmostEqual(obs[1], 0.08)
        env = rs.SlayEnv(deck=["Entrench+"])
        obs, _ = env.reset(0)
        self.assertAlmostEqual(obs[9], 1 / 3)
        env = rs.SlayEnv(deck=["BodySlam+"])
        obs, _ = env.reset(0)
        self.assertEqual(obs[9], 0)

    def test_invalid_and_unimplemented_choices_fail_at_construction(self):
        for cards in ([], ["strike"], ["Strike++"], ["Unknown"], ["Dazed+"],
                      ["Armaments"], ["Armaments+"], ["TrueGrit+"], ["Warcry"], ["Headbutt"]):
            with self.subTest(cards=cards), self.assertRaises(ValueError):
                rs.SlayEnv(deck=cards)
        with self.assertRaises(TypeError):
            rs.SlayEnv(deck=[3])
        self.assertEqual(rs.SlayEnv(deck=["TrueGrit"]).deck, ["TrueGrit"])

    def test_cli_deck_selection_and_json_validation(self):
        parser = argparse.ArgumentParser()
        add_deck_arguments(parser)
        self.assertEqual(resolve_deck(parser.parse_args([])), list(STARTER))
        self.assertEqual(resolve_deck(parser.parse_args(["--deck-preset", "act1_late"])), list(PRESETS["act1_late"]))
        self.assertEqual(resolve_deck(parser.parse_args(["--deck", "Strike+", "Defend"])), ["Strike+", "Defend"])
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "deck.json"
            path.write_text('["Bash+", "Inflame"]')
            args = parser.parse_args(["--deck-file", str(path)])
            self.assertEqual(resolve_deck(args), ["Bash+", "Inflame"])
            for value in ({"cards": ["Strike"]}, [], [2]):
                path.write_text(json.dumps(value))
                with self.assertRaises(ValueError):
                    resolve_deck(args)

    def test_checkpoint_deck_metadata_and_legacy_fallback(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "interface.json"
            model = Path(directory) / "final.zip"
            with self.assertRaisesRegex(ValueError, "specify a deck"):
                model_deck(model)
            path.write_text(json.dumps({"deck": ["Strike+", "Inflame+"]}))
            self.assertEqual(model_deck(model), ["Strike+", "Inflame+"])
            path.write_text('{"interface_version": 4}')
            self.assertEqual(model_deck(model), list(STARTER))
            path.write_text('[]')
            with self.assertRaisesRegex(ValueError, "object"):
                model_deck(model)

    @unittest.skipUnless(importlib.util.find_spec("gymnasium"), "requires train dependencies")
    def test_presets_complete_seeded_elite_and_boss_episodes(self):
        from spire_env import SpireEnv
        policy = random.Random(1234)
        for name, deck in PRESETS.items():
            for enemy in ("Lagavulin", "TheGuardian"):
                env = SpireEnv(enemy=enemy, ascension=0, deck=list(deck))
                self.assertEqual(env.deck, list(deck))
                for seed in range(5):
                    obs, _ = env.reset(seed=seed)
                    for _ in range(1000):
                        self.assertTrue(env.observation_space.contains(obs))
                        legal = [i for i, enabled in enumerate(env.action_masks()) if enabled]
                        obs, _, done, truncated, _ = env.step(policy.choice(legal))
                        self.assertFalse(truncated, (name, enemy, seed))
                        if done:
                            break
                    else:
                        self.fail((name, enemy, seed))
                env.close()


if __name__ == "__main__":
    unittest.main()
