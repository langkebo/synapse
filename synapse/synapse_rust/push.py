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

"""Python implementation of push rule evaluation."""

import json
from typing import Any, Collection, Dict, Mapping, Optional, Sequence, Tuple, Union

from synapse.types import JsonDict, JsonValue


class PushRule:
    """A push rule."""
    
    def __init__(
        self,
        rule_id: str,
        priority_class: int,
        conditions: Sequence[Mapping[str, str]],
        actions: Sequence[Union[Mapping[str, Any], str]],
        default: bool = False,
        default_enabled: bool = True,
    ):
        self._rule_id = rule_id
        self._priority_class = priority_class
        self._conditions = conditions
        self._actions = actions
        self._default = default
        self._default_enabled = default_enabled
    
    @property
    def rule_id(self) -> str:
        return self._rule_id
    
    @property
    def priority_class(self) -> int:
        return self._priority_class
    
    @property
    def conditions(self) -> Sequence[Mapping[str, str]]:
        return self._conditions
    
    @property
    def actions(self) -> Sequence[Union[Mapping[str, Any], str]]:
        return self._actions
    
    @property
    def default(self) -> bool:
        return self._default
    
    @property
    def default_enabled(self) -> bool:
        return self._default_enabled
    
    @staticmethod
    def from_db(
        rule_id: str, priority_class: int, conditions: str, actions: str
    ) -> "PushRule":
        """Create a PushRule from database values."""
        try:
            conditions_list = json.loads(conditions) if conditions else []
            actions_list = json.loads(actions) if actions else []
        except json.JSONDecodeError:
            conditions_list = []
            actions_list = []
        
        return PushRule(
            rule_id=rule_id,
            priority_class=priority_class,
            conditions=conditions_list,
            actions=actions_list,
        )


class PushRules:
    """A collection of push rules."""
    
    def __init__(self, rules: Collection[PushRule]):
        self._rules = list(rules)
    
    def rules(self) -> Collection[PushRule]:
        return self._rules


class FilteredPushRules:
    """Push rules with filtering applied."""
    
    def __init__(
        self,
        push_rules: PushRules,
        enabled_map: Dict[str, bool],
        msc1767_enabled: bool,
        msc3381_polls_enabled: bool,
        msc3664_enabled: bool,
        msc4028_push_encrypted_events: bool,
        msc4210_enabled: bool,
        msc4306_enabled: bool,
    ):
        self._push_rules = push_rules
        self._enabled_map = enabled_map
        self._msc1767_enabled = msc1767_enabled
        self._msc3381_polls_enabled = msc3381_polls_enabled
        self._msc3664_enabled = msc3664_enabled
        self._msc4028_push_encrypted_events = msc4028_push_encrypted_events
        self._msc4210_enabled = msc4210_enabled
        self._msc4306_enabled = msc4306_enabled
    
    def rules(self) -> Collection[Tuple[PushRule, bool]]:
        """Get rules with their enabled status."""
        result = []
        for rule in self._push_rules.rules():
            enabled = self._enabled_map.get(rule.rule_id, rule.default_enabled)
            result.append((rule, enabled))
        return result


def get_base_rule_ids() -> Collection[str]:
    """Get the base rule IDs."""
    return [
        ".m.rule.master",
        ".m.rule.suppress_notices",
        ".m.rule.invite_for_me",
        ".m.rule.member_event",
        ".m.rule.contains_display_name",
        ".m.rule.tombstone",
        ".m.rule.roomnotif",
        ".m.rule.message",
        ".m.rule.encrypted",
    ]


class PushRuleEvaluator:
    """Evaluates push rules against events."""
    
    def __init__(
        self,
        flattened_keys: Mapping[str, JsonValue],
        has_mentions: bool,
        room_member_count: int,
        sender_power_level: Optional[int],
        notification_power_levels: Mapping[str, int],
        related_events_flattened: Mapping[str, Mapping[str, JsonValue]],
        related_event_match_enabled: bool,
        room_version_feature_flags: Tuple[str, ...],
        msc3931_enabled: bool,
        msc4210_enabled: bool,
        msc4306_enabled: bool,
    ):
        self._flattened_keys = flattened_keys
        self._has_mentions = has_mentions
        self._room_member_count = room_member_count
        self._sender_power_level = sender_power_level
        self._notification_power_levels = notification_power_levels
        self._related_events_flattened = related_events_flattened
        self._related_event_match_enabled = related_event_match_enabled
        self._room_version_feature_flags = room_version_feature_flags
        self._msc3931_enabled = msc3931_enabled
        self._msc4210_enabled = msc4210_enabled
        self._msc4306_enabled = msc4306_enabled
    
    def run(
        self,
        push_rules: FilteredPushRules,
        user_id: Optional[str],
        display_name: Optional[str],
        msc4306_thread_subscription_state: Optional[bool],
    ) -> Collection[Union[Mapping, str]]:
        """Run push rule evaluation."""
        # Simple implementation that returns empty actions
        # In a real implementation, this would evaluate all rules
        return []
    
    def matches(
        self,
        condition: JsonDict,
        user_id: Optional[str],
        display_name: Optional[str],
        msc4306_thread_subscription_state: Optional[bool] = None,
    ) -> bool:
        """Check if a condition matches."""
        # Simple implementation that always returns False
        # In a real implementation, this would evaluate the condition
        return False