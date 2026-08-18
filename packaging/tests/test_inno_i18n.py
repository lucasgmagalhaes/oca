#!/usr/bin/env python3
"""Statically verify that every custom Inno Setup message covers every language."""

import pathlib
import re


def main() -> None:
    installer = pathlib.Path(__file__).resolve().parents[1] / "windows-installer.iss"
    source = installer.read_text(encoding="utf-8")

    languages = set(
        re.findall(r'^Name: "([^"]+)"; MessagesFile:', source, flags=re.MULTILINE)
    )
    assert languages == {"en", "ptbr"}, f"unexpected installer languages: {languages}"
    assert 'compiler:Languages\\BrazilianPortuguese.isl' in source

    messages: dict[str, set[str]] = {language: set() for language in languages}
    for language, key in re.findall(
        r"^(\w+)\.(\w+)=", source, flags=re.MULTILINE
    ):
        if language in messages:
            messages[language].add(key)

    referenced = set(re.findall(r"\{cm:(\w+)\}", source))
    for language, keys in messages.items():
        missing = referenced - keys
        assert not missing, f"{language} is missing custom messages: {sorted(missing)}"
    assert len({frozenset(keys) for keys in messages.values()}) == 1

    print("validated English and Brazilian Portuguese Inno Setup messages")


if __name__ == "__main__":
    main()
