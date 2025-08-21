# Copyright 2022 The Matrix.org Foundation C.I.C.
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

"""Python implementation of text segmentation."""

import re
from typing import List


def parse_words(text: str) -> List[str]:
    """Parse words from text.
    
    This is a simple implementation that splits on whitespace and punctuation.
    In a real implementation, this would use ICU segmentation.
    
    Args:
        text: The text to parse
        
    Returns:
        A list of words
    """
    if not text:
        return []
    
    # Simple word segmentation using regex
    # Split on whitespace and punctuation, keeping only alphanumeric sequences
    words = re.findall(r'\b\w+\b', text.lower())
    return words