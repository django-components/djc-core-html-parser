# This file is what maturin auto-generates. But it seems maturin omits it when we have a __init__.pyi file.
# So we have to manually include it here.
# Following block of code is what maturin would've generated

from .djc_core import *

__doc__ = djc_core.__doc__
if hasattr(djc_core, "__all__"):
    __all__ = djc_core.__all__

# OVERRIDES START HERE
# Add here any additional public API that we defined purely in Python
from djc_core.djc_template_parser import CompiledFunc, compile_tag

if hasattr(djc_core, "__all__"):
    __all__ += ["CompiledFunc", "compile_tag"]
