# Copyright 2015, 2016 OpenMarket Ltd
# Copyright 2019 The Matrix.org Foundation C.I.C.
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

"""缓存模块初始化文件

Cache module initialization file
"""

from enum import Enum
from typing import Any, Callable, Dict, Optional, TypeVar

T = TypeVar("T", Optional[str], str)

# Track memory usage for caches
TRACK_MEMORY_USAGE = True

# Define missing classes and functions
class EvictionReason(Enum):
    """Reasons for cache eviction."""
    SIZE = "size"
    TIME = "time"
    INVALIDATION = "invalidation"

class CacheMetric:
    """Metrics for cache performance."""
    def __init__(self, name: str):
        self.name = name
        self.hits = 0
        self.misses = 0
        self.evictions = 0
    
    def record_hit(self) -> None:
        self.hits += 1
    
    def record_miss(self) -> None:
        self.misses += 1
    
    def record_eviction(self, reason: EvictionReason) -> None:
        self.evictions += 1

def register_cache(name: Optional[str] = None, cache: Optional[Any] = None, collect_callback: Optional[Callable] = None, cache_type: Optional[str] = None, cache_name: Optional[str] = None, server_name: Optional[str] = None, resize_callback: Optional[Callable] = None, **kwargs) -> CacheMetric:
    """Register a cache for monitoring."""
    metric = CacheMetric(cache_name or name or "unknown")
    # In a real implementation, this would register with a metrics system
    return metric

# Known keys for interning
KNOWN_KEYS = {
    key: key
    for key in (
        "auth_events",
        "content",
        "depth",
        "event_id",
        "hashes",
        "origin",
        "origin_server_ts",
        "prev_events",
        "room_id",
        "sender",
        "signatures",
        "state_key",
        "type",
        "unsigned",
        "user_id",
    )
}

def intern_string(string: T) -> T:
    """Takes a (potentially) unicode string and interns it if it's ascii"""
    if string is None:
        return None

    try:
        return intern(string)
    except UnicodeEncodeError:
        return string

def intern_dict(dictionary: Dict[str, Any]) -> Dict[str, Any]:
    """Takes a dictionary and interns well known keys and their values"""
    return {
        KNOWN_KEYS.get(key, key): _intern_known_values(key, value)
        for key, value in dictionary.items()
    }

def _intern_known_values(key: str, value: Any) -> Any:
    intern_keys = ("event_id", "room_id", "sender", "user_id", "type", "state_key")

    if key in intern_keys:
        return intern_string(value)

    return value

from .cache_manager import CacheManager
from .descriptors import cached, cachedList
from .lrucache import LruCache
from .response_cache import ResponseCache
from .stream_change_cache import StreamChangeCache
from .ttlcache import TTLCache

__all__ = [
    "CacheManager",
    "CacheMetric",
    "EvictionReason",
    "cached",
    "cachedList", 
    "intern_dict",
    "intern_string",
    "LruCache",
    "register_cache",
    "ResponseCache",
    "StreamChangeCache",
    "TTLCache",
]