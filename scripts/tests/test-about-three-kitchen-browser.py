#!/usr/bin/env python3
"""Compatibility entry point: the first-kitchen check now covers the whole tour."""
import runpy
from pathlib import Path
runpy.run_path(str(Path(__file__).with_name('test-about-three-tour-browser.py')),run_name='__main__')
