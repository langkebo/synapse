# This file is licensed under the Affero General Public License (AGPL) version 3.
#
# Copyright (C) 2025 New Vector, Ltd
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
#
# See the GNU Affero General Public License for more details:
# <https://www.gnu.org/licenses/agpl-3.0.html>.

"""Python implementation of HTTP client."""

from typing import Mapping

from twisted.internet import defer
from twisted.internet.defer import Deferred
from twisted.web.client import Agent, readBody
from twisted.web.http_headers import Headers
from twisted.web.iweb import IBodyProducer
from zope.interface import implementer

from synapse.types import ISynapseReactor


@implementer(IBodyProducer)
class StringProducer:
    """A body producer for string data."""
    
    def __init__(self, body: str):
        self.body = body.encode('utf-8')
        self.length = len(self.body)
    
    def startProducing(self, consumer):
        consumer.write(self.body)
        return defer.succeed(None)
    
    def pauseProducing(self):
        pass
    
    def stopProducing(self):
        pass


class HttpClient:
    """HTTP client implementation."""
    
    def __init__(self, reactor: ISynapseReactor, user_agent: str) -> None:
        """Initialize the HTTP client.
        
        Args:
            reactor: The Twisted reactor
            user_agent: User agent string
        """
        self._reactor = reactor
        self._user_agent = user_agent
        self._agent = Agent(reactor)
    
    def get(self, url: str, response_limit: int) -> Deferred[bytes]:
        """Perform a GET request.
        
        Args:
            url: The URL to request
            response_limit: Maximum response size in bytes
            
        Returns:
            Deferred that fires with the response body
        """
        headers = Headers({
            b'user-agent': [self._user_agent.encode('utf-8')]
        })
        
        d = self._agent.request(b'GET', url.encode('utf-8'), headers)
        d.addCallback(self._handle_response, response_limit)
        return d
    
    def post(
        self,
        url: str,
        response_limit: int,
        headers: Mapping[str, str],
        request_body: str,
    ) -> Deferred[bytes]:
        """Perform a POST request.
        
        Args:
            url: The URL to request
            response_limit: Maximum response size in bytes
            headers: Request headers
            request_body: Request body
            
        Returns:
            Deferred that fires with the response body
        """
        # Convert headers to Twisted format
        twisted_headers = Headers({
            b'user-agent': [self._user_agent.encode('utf-8')]
        })
        
        for key, value in headers.items():
            twisted_headers.addRawHeader(key.encode('utf-8'), value.encode('utf-8'))
        
        # Create body producer
        body_producer = StringProducer(request_body)
        
        d = self._agent.request(
            b'POST',
            url.encode('utf-8'),
            twisted_headers,
            body_producer
        )
        d.addCallback(self._handle_response, response_limit)
        return d
    
    def _handle_response(self, response, response_limit: int) -> Deferred[bytes]:
        """Handle the HTTP response.
        
        Args:
            response: The HTTP response
            response_limit: Maximum response size
            
        Returns:
            Deferred that fires with the response body
        """
        # Check content length if available
        content_length = response.headers.getRawHeaders(b'content-length')
        if content_length:
            try:
                length = int(content_length[0])
                if length > response_limit:
                    return defer.fail(Exception(f"Response too large: {length} > {response_limit}"))
            except (ValueError, IndexError):
                pass
        
        # Read the response body
        d = readBody(response)
        d.addCallback(self._check_response_size, response_limit)
        return d
    
    def _check_response_size(self, body: bytes, response_limit: int) -> bytes:
        """Check if the response body is within the size limit.
        
        Args:
            body: The response body
            response_limit: Maximum response size
            
        Returns:
            The response body if within limits
            
        Raises:
            Exception: If the response is too large
        """
        if len(body) > response_limit:
            raise Exception(f"Response too large: {len(body)} > {response_limit}")
        return body