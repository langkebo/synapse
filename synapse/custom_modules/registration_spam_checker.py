# -*- coding: utf-8 -*-
"""
Custom Registration Spam Checker module for Synapse

This module demonstrates how to use ModuleApi.register_spam_checker_callbacks
and implements basic, configurable checks for registration requests.

Configuration example to put into homeserver.yaml:

modules:
  - module: synapse.custom_modules.registration_spam_checker.RegistrationSpamChecker
    config:
      deny_email_domains: ["mailinator.com", "10minutemail.com", "guerrillamail.com"]
      allow_email_domains: []
      deny_username_regexes: ["^admin$", "^root$", "^support$"]
      min_username_length: 3
      max_username_length: 64
      require_user_agent: true
      shadow_ban_instead_of_deny: false

"""
import logging
import re
from typing import Collection, List, Optional, Tuple

from synapse.module_api import ModuleApi
from synapse.spam_checker_api import RegistrationBehaviour

logger = logging.getLogger(__name__)


class RegistrationSpamChecker:
    def __init__(self, config: dict, api: ModuleApi) -> None:
        self._api = api
        self._config = config or {}

        self._deny_email_domains: List[str] = [
            d.lower() for d in self._config.get("deny_email_domains", [])
        ]
        self._allow_email_domains: List[str] = [
            d.lower() for d in self._config.get("allow_email_domains", [])
        ]
        self._deny_username_regexes: List[re.Pattern] = [
            re.compile(p, flags=re.IGNORECASE)
            for p in self._config.get(
                "deny_username_regexes",
                [r"^admin$", r"^root$", r"^support$", r"^postmaster$"],
            )
        ]
        self._min_username_length: int = int(self._config.get("min_username_length", 3))
        self._max_username_length: int = int(self._config.get("max_username_length", 64))
        self._require_user_agent: bool = bool(self._config.get("require_user_agent", True))
        self._shadow_ban_instead_of_deny: bool = bool(
            self._config.get("shadow_ban_instead_of_deny", False)
        )

        # Register callbacks with Synapse.
        api.register_spam_checker_callbacks(
            check_registration_for_spam=self.check_registration_for_spam
        )

        logger.info(
            "RegistrationSpamChecker loaded (deny_email_domains=%s, allow_email_domains=%s, min_len=%s, max_len=%s)",
            self._deny_email_domains,
            self._allow_email_domains,
            self._min_username_length,
            self._max_username_length,
        )

    @staticmethod
    def parse_config(config: Optional[dict]) -> dict:
        # Minimal passthrough; could be extended with jsonschema if needed
        return config or {}

    async def check_registration_for_spam(
        self,
        email_threepid: Optional[dict],
        username: Optional[str],
        request_info: Collection[Tuple[str, str]],
        auth_provider_id: Optional[str] = None,
    ) -> RegistrationBehaviour:
        """Implement registration spam checks.

        Returns ALLOW unless a rule triggers DENY or SHADOW_BAN.
        """
        # 1) Validate username constraints
        if username:
            u = username.strip()
            if len(u) < self._min_username_length or len(u) > self._max_username_length:
                return self._behaviour()
            for pat in self._deny_username_regexes:
                if pat.search(u):
                    return self._behaviour()

        # 2) Validate email domain allow/deny lists
        if email_threepid and isinstance(email_threepid, dict):
            address = (email_threepid.get("address") or "").lower().strip()
            if address and "@" in address:
                domain = address.split("@", 1)[1]
                if self._allow_email_domains and domain not in self._allow_email_domains:
                    return self._behaviour()
                if domain in self._deny_email_domains:
                    return self._behaviour()

        # 3) Basic user-agent/IP checks: optionally require at least one UA
        if self._require_user_agent:
            has_ua = False
            for ua, _ip in request_info:
                if ua:
                    has_ua = True
                    break
            if not has_ua:
                return self._behaviour()

        # 4) All checks passed
        return RegistrationBehaviour.ALLOW

    def _behaviour(self) -> RegistrationBehaviour:
        return (
            RegistrationBehaviour.SHADOW_BAN
            if self._shadow_ban_instead_of_deny
            else RegistrationBehaviour.DENY
        )