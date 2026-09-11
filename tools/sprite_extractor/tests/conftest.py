import os
import sys

# Tests import the package's flat top-level modules (`config`, `utils.*`,
# `detection.*`, ...) directly, so the package root needs to be on
# sys.path - it isn't automatically when pytest is invoked from the repo
# root (`pytest tools/sprite_extractor/tests`).
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
