# This file is licensed under the Affero General Public License (AGPL) version 3.
#
# Copyright (C) 2024 New Vector, Ltd
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
#
# See the GNU Affero General Public License for more details:
# <https://www.gnu.org/licenses/agpl-3.0.html>.

"""Python implementations of Rust modules for Synapse."""

def reset_logging_config() -> None:
    """Reset the logging configuration.
    
    This is a Python implementation of the Rust function that would
    reset the pyo3-log cache.
    """
    # In a real Rust implementation, this would reset the pyo3-log cache
    # For now, this is a no-op placeholder
    pass