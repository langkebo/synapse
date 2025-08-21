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

from typing import List, Mapping, Optional, Tuple

from synapse.types import JsonDict


class EventInternalMetadata:
    """Internal metadata for events."""
    
    def __init__(self, internal_metadata_dict: JsonDict):
        self._dict = internal_metadata_dict.copy()
        
        # Initialize properties with default values
        self.stream_ordering: Optional[int] = self._dict.get("stream_ordering")
        self.instance_name: Optional[str] = self._dict.get("instance_name")
        self.outlier: bool = self._dict.get("outlier", False)
        self.out_of_band_membership: bool = self._dict.get("out_of_band_membership", False)
        self.send_on_behalf_of: str = self._dict.get("send_on_behalf_of", "")
        self.recheck_redaction: bool = self._dict.get("recheck_redaction", False)
        self.soft_failed: bool = self._dict.get("soft_failed", False)
        self.proactively_send: bool = self._dict.get("proactively_send", True)
        self.redacted: bool = self._dict.get("redacted", False)
        self.policy_server_spammy: bool = self._dict.get("policy_server_spammy", False)
        self.txn_id: str = self._dict.get("txn_id", "")
        self.token_id: int = self._dict.get("token_id", 0)
        self.device_id: str = self._dict.get("device_id", "")
    
    def get_dict(self) -> JsonDict:
        """Get the internal metadata as a dictionary."""
        return self._dict.copy()
    
    def is_outlier(self) -> bool:
        """Whether this event is an outlier."""
        return self.outlier
    
    def copy(self) -> "EventInternalMetadata":
        """Create a copy of this metadata."""
        return EventInternalMetadata(self._dict)
    
    def is_out_of_band_membership(self) -> bool:
        """Whether this event is an out-of-band membership.
        
        OOB memberships are a special case of outlier events: they are membership events
        for federated rooms that we aren't full members of. Examples include invites
        received over federation, and rejections for such invites.
        
        The concept of an OOB membership is needed because these events need to be
        processed as if they're new regular events (e.g. updating membership state in
        the database, relaying to clients via /sync, etc) despite being outliers.
        
        See also https://element-hq.github.io/synapse/develop/development/room-dag-concepts.html#out-of-band-membership-events.
        
        (Added in synapse 0.99.0, so may be unreliable for events received before that)
        """
        return self.out_of_band_membership
    
    def get_send_on_behalf_of(self) -> Optional[str]:
        """Whether this server should send the event on behalf of another server.
        This is used by the federation "send_join" API to forward the initial join
        event for a server in the room.
        
        returns a str with the name of the server this event is sent on behalf of.
        """
        return self.send_on_behalf_of if self.send_on_behalf_of else None
    
    def need_to_check_redaction(self) -> bool:
        """Whether the redaction event needs to be rechecked when fetching
        from the database.
        
        Starting in room v3 redaction events are accepted up front, and later
        checked to see if the redacter and redactee's domains match.
        
        If the sender of the redaction event is allowed to redact any event
        due to auth rules, then this will always return false.
        """
        return self.recheck_redaction
    
    def is_soft_failed(self) -> bool:
        """Whether the event has been soft failed.
        
        Soft failed events should be handled as usual, except:
            1. They should not go down sync or event streams, or generally
               sent to clients.
            2. They should not be added to the forward extremities (and
               therefore not to current state).
        """
        return self.soft_failed
    
    def should_proactively_send(self) -> bool:
        """Whether the event, if ours, should be sent to other clients and
        servers.
        
        This is used for sending dummy events internally. Servers and clients
        can still explicitly fetch the event.
        """
        return self.proactively_send
    
    def is_redacted(self) -> bool:
        """Whether the event has been redacted.
        
        This is used for efficiently checking whether an event has been
        marked as redacted without needing to make another database call.
        """
        return self.redacted
    
    def is_notifiable(self) -> bool:
        """Whether this event can trigger a push notification"""
        # Default implementation - can be overridden based on specific logic
        return not self.is_soft_failed() and self.should_proactively_send()


def event_visible_to_server(
    sender: str,
    target_server_name: str,
    history_visibility: str,
    erased_senders: Mapping[str, bool],
    partial_state_invisible: bool,
    memberships: List[Tuple[str, str]],
) -> bool:
    """Determine whether the server is allowed to see the unredacted event.
    
    Args:
        sender: The sender of the event.
        target_server_name: The server we want to send the event to.
        history_visibility: The history_visibility value at the event.
        erased_senders: A mapping of users and whether they have requested erasure. If a
            user is not in the map, it is treated as though they haven't requested erasure.
        partial_state_invisible: Whether the event should be treated as invisible due to
            the partial state status of the room.
        memberships: A list of membership state information at the event for users
            matching the `target_server_name`. Each list item must contain a tuple of
            (state_key, membership).
    
    Returns:
        Whether the server is allowed to see the unredacted event.
    """
    # If the event is invisible due to partial state, deny access
    if partial_state_invisible:
        return False
    
    # Check if sender has been erased
    if erased_senders.get(sender, False):
        return False
    
    # Basic visibility logic based on history_visibility
    if history_visibility == "world_readable":
        return True
    elif history_visibility == "shared":
        # Check if target server has any members in the room
        return len(memberships) > 0
    elif history_visibility == "invited":
        # Check if target server has invited or joined members
        for _, membership in memberships:
            if membership in ["invite", "join"]:
                return True
        return False
    elif history_visibility == "joined":
        # Check if target server has joined members
        for _, membership in memberships:
            if membership == "join":
                return True
        return False
    
    # Default to not visible
    return False