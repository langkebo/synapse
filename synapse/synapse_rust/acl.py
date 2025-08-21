# Copyright 2023 The Matrix.org Foundation C.I.C.
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

"""Python implementation of server ACL evaluation."""

import fnmatch
import ipaddress
from typing import List


class ServerAclEvaluator:
    """Evaluates server ACL rules."""
    
    def __init__(
        self, allow_ip_literals: bool, allow: List[str], deny: List[str]
    ) -> None:
        """Initialize the ACL evaluator.
        
        Args:
            allow_ip_literals: Whether to allow IP literals
            allow: List of allowed server patterns
            deny: List of denied server patterns
        """
        self.allow_ip_literals = allow_ip_literals
        self.allow = allow or []
        self.deny = deny or []
    
    def server_matches_acl_event(self, server_name: str) -> bool:
        """Check if a server name matches the ACL rules.
        
        Args:
            server_name: The server name to check
            
        Returns:
            True if the server is allowed, False if denied
        """
        if not server_name:
            return False
        
        # Check if it's an IP literal
        is_ip_literal = self._is_ip_literal(server_name)
        if is_ip_literal and not self.allow_ip_literals:
            return False
        
        # Check deny list first
        for deny_pattern in self.deny:
            if self._matches_pattern(server_name, deny_pattern):
                return False
        
        # Check allow list
        if not self.allow:
            # If no allow list, allow by default (unless denied above)
            return True
        
        for allow_pattern in self.allow:
            if self._matches_pattern(server_name, allow_pattern):
                return True
        
        # Not in allow list
        return False
    
    def _is_ip_literal(self, server_name: str) -> bool:
        """Check if the server name is an IP literal."""
        # Remove port if present
        host = server_name.split(':')[0]
        
        try:
            ipaddress.ip_address(host)
            return True
        except ValueError:
            return False
    
    def _matches_pattern(self, server_name: str, pattern: str) -> bool:
        """Check if server name matches a pattern.
        
        Supports glob-style patterns with * and ?.
        """
        return fnmatch.fnmatch(server_name.lower(), pattern.lower())