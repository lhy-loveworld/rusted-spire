"""Explicit combat deck snapshots; these do not simulate card rewards or a run."""
import json
from pathlib import Path

STARTER = ("Strike",) * 5 + ("Defend",) * 4 + ("Bash",)
PRESETS = {
    "starter": STARTER,
    "act1_early": STARTER + ("TwinStrike", "ShrugItOff"),
    "act1_mid": ("Strike",) * 5 + ("Defend",) * 4
        + ("Bash+", "TwinStrike", "ShrugItOff", "Cleave", "Inflame"),
    "act1_late": ("Strike",) * 5 + ("Defend",) * 4
        + ("Bash+", "TwinStrike+", "ShrugItOff+", "Cleave", "Inflame", "HeavyBlade", "Metallicize"),
}


def add_deck_arguments(parser, *, default_preset="starter"):
    group = parser.add_mutually_exclusive_group()
    group.add_argument("--deck-preset", choices=tuple(PRESETS), default=default_preset)
    group.add_argument("--deck-file", type=Path, help='JSON array of card names, e.g. ["Strike", "Bash+"]')
    group.add_argument("--deck", nargs="+", help="explicit ordered card names; suffix + upgrades a card")


def resolve_deck(args):
    if args.deck is not None:
        cards = args.deck
    elif args.deck_file is not None:
        cards = json.loads(args.deck_file.read_text())
    elif args.deck_preset is not None:
        cards = PRESETS[args.deck_preset]
    else:
        return None
    if not isinstance(cards, (list, tuple)) or not cards or not all(isinstance(c, str) for c in cards):
        raise ValueError("deck must be a nonempty JSON array/list of card-name strings")
    # Rust is the authority for names, upgrades and unsupported selections.
    import rusted_spire
    return rusted_spire.SlayEnv(deck=list(cards)).deck


def model_deck(model_path):
    """Use saved deck metadata when present; legacy checkpoints used the starter."""
    metadata = Path(model_path).parent / "interface.json"
    if not metadata.exists():
        raise ValueError("checkpoint lacks interface.json; specify a deck explicitly")
    data = json.loads(metadata.read_text())
    if not isinstance(data, dict):
        raise ValueError("checkpoint interface.json must contain an object")
    cards = data.get("deck", list(STARTER))
    if not isinstance(cards, list) or not cards or not all(isinstance(c, str) for c in cards):
        raise ValueError("invalid deck in checkpoint interface.json")
    import rusted_spire
    return rusted_spire.SlayEnv(deck=cards).deck
