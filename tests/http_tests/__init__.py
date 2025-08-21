#
# This file is licensed under the Affero General Public License (AGPL) version 3.
#
# Copyright (C) 2023 New Vector, Ltd
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as
# published by the Free Software Foundation, either version 3 of the
# License, or (at your option) any later version.
#
# See the GNU Affero General Public License for more details:
# <https://www.gnu.org/licenses/agpl-3.0.html>.
#
# Originally licensed under the Apache License, Version 2.0:
# <http://www.apache.org/licenses/LICENSE-2.0>.
#
# [This file includes modifications made by New Vector Limited]
#
#

# Prevent this module from being imported as the standard library 'http' module
import sys
if __name__ == 'http':
    # If we're being imported as 'http', redirect to the standard library
    import importlib
    import http as _stdlib_http
    sys.modules[__name__] = _stdlib_http
    raise ImportError("Redirecting to standard library http module")

import os.path
import subprocess
from typing import List

# Avoid circular imports by importing twisted modules only when needed
# from incremental import Version
# from zope.interface import implementer
# import twisted
# from OpenSSL import SSL
# from OpenSSL.SSL import Connection
# from twisted.internet.interfaces import (
#     IOpenSSLServerConnectionCreator,
#     IProtocolFactory,
#     IReactorTime,
# )
# from twisted.internet.ssl import Certificate, trustRootFromCertificates
# from twisted.protocols.tls import TLSMemoryBIOFactory, TLSMemoryBIOProtocol
# from twisted.web.client import BrowserLikePolicyForHTTPS  # noqa: F401
# from twisted.web.iweb import IPolicyForHTTPS  # noqa: F401


def get_test_https_policy():
    """Get a test IPolicyForHTTPS which trusts the test CA cert

    Returns:
        IPolicyForHTTPS
    """
    from twisted.internet.ssl import Certificate, trustRootFromCertificates
    from twisted.web.client import BrowserLikePolicyForHTTPS
    
    ca_file = get_test_ca_cert_file()
    with open(ca_file) as stream:
        content = stream.read()
    cert = Certificate.loadPEM(content)
    trust_root = trustRootFromCertificates([cert])
    return BrowserLikePolicyForHTTPS(trustRoot=trust_root)


def get_test_ca_cert_file() -> str:
    """Get the path to the test CA cert

    The keypair is generated with:

        openssl genrsa -out ca.key 2048
        openssl req -new -x509 -key ca.key -days 3650 -out ca.crt \
            -subj '/CN=synapse test CA'
    """
    return os.path.join(os.path.dirname(__file__), "ca.crt")


def get_test_key_file() -> str:
    """get the path to the test key

    The key file is made with:

        openssl genrsa -out server.key 2048
    """
    return os.path.join(os.path.dirname(__file__), "server.key")


cert_file_count = 0

CONFIG_TEMPLATE = b"""\
[default]
basicConstraints = CA:FALSE
keyUsage=nonRepudiation, digitalSignature, keyEncipherment
subjectAltName = %(sanentries)s
"""


def create_test_cert_file(sanlist: List[bytes]) -> str:
    """build an x509 certificate file

    Args:
        sanlist: a list of subjectAltName values for the cert

    Returns:
        The path to the file
    """
    global cert_file_count
    csr_filename = "server.csr"
    cnf_filename = "server.%i.cnf" % (cert_file_count,)
    cert_filename = "server.%i.crt" % (cert_file_count,)
    cert_file_count += 1

    # first build a CSR
    subprocess.check_call(
        [
            "openssl",
            "req",
            "-new",
            "-key",
            get_test_key_file(),
            "-subj",
            "/",
            "-out",
            csr_filename,
        ]
    )

    # now a config file describing the right SAN entries
    sanentries = b",".join(sanlist)
    with open(cnf_filename, "wb") as f:
        f.write(CONFIG_TEMPLATE % {b"sanentries": sanentries})

    # finally the cert
    ca_key_filename = os.path.join(os.path.dirname(__file__), "ca.key")
    ca_cert_filename = get_test_ca_cert_file()
    subprocess.check_call(
        [
            "openssl",
            "x509",
            "-req",
            "-in",
            csr_filename,
            "-CA",
            ca_cert_filename,
            "-CAkey",
            ca_key_filename,
            "-set_serial",
            "1",
            "-extfile",
            cnf_filename,
            "-out",
            cert_filename,
        ]
    )

    return cert_filename


class TestServerTLSConnectionFactory:
    """An SSL connection creator which returns connections which present a certificate
    signed by our test CA."""

    def __init__(self, sanlist: List[bytes]):
        """
        Args:
            sanlist: a list of subjectAltName values for the cert
        """
        self._cert_file = create_test_cert_file(sanlist)

    def serverConnectionForTLS(self, tlsProtocol):
        from zope.interface import implementer
        from twisted.internet.interfaces import IOpenSSLServerConnectionCreator
        from OpenSSL import SSL
        from OpenSSL.SSL import Connection
        
        # Apply the implementer decorator dynamically
        if not hasattr(self.__class__, '_decorated'):
            self.__class__ = implementer(IOpenSSLServerConnectionCreator)(self.__class__)
            self.__class__._decorated = True
            
        ctx = SSL.Context(SSL.SSLv23_METHOD)
        ctx.use_certificate_file(self._cert_file)
        ctx.use_privatekey_file(get_test_key_file())
        return Connection(ctx, None)


def wrap_server_factory_for_tls(
    factory, clock, sanlist: List[bytes]
):
    """Wrap an existing Protocol Factory with a test TLSMemoryBIOFactory

    The resultant factory will create a TLS server which presents a certificate
    signed by our test CA, valid for the domains in `sanlist`

    Args:
        factory: protocol factory to wrap
        sanlist: list of domains the cert should be valid for

    Returns:
        interfaces.IProtocolFactory
    """
    import twisted
    from incremental import Version
    from twisted.protocols.tls import TLSMemoryBIOFactory
    
    connection_creator = TestServerTLSConnectionFactory(sanlist=sanlist)
    # Twisted > 23.8.0 has a different API that accepts a clock.
    if twisted.version <= Version("Twisted", 23, 8, 0):
        return TLSMemoryBIOFactory(
            connection_creator, isClient=False, wrappedFactory=factory
        )
    else:
        return TLSMemoryBIOFactory(
            connection_creator, isClient=False, wrappedFactory=factory, clock=clock
        )


# A dummy address, useful for tests that use FakeTransport and don't care about where
# packets are going to/coming from.
def get_dummy_address():
    """Get a dummy IPv4Address for testing purposes."""
    from twisted.internet.address import IPv4Address
    return IPv4Address("TCP", "127.0.0.1", 80)

# Create a lazy property for backward compatibility
class _DummyAddressProperty:
    def __init__(self):
        self._address = None
    
    def __getattr__(self, name):
        if self._address is None:
            self._address = get_dummy_address()
        return getattr(self._address, name)
    
    def __str__(self):
        if self._address is None:
            self._address = get_dummy_address()
        return str(self._address)
    
    def __repr__(self):
        if self._address is None:
            self._address = get_dummy_address()
        return repr(self._address)

dummy_address = _DummyAddressProperty()
