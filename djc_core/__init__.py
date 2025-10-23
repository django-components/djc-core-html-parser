# This file is what maturin auto-generates. But it seems maturin omits it when we have a __init__.pyi file.
# So we have to manually include it here.
# Following block of code is what maturin would've generated

from .djc_core import *

__doc__ = djc_core.__doc__
if hasattr(djc_core, "__all__"):
    __all__ = djc_core.__all__
