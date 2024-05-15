// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at http://mozilla.org/MPL/2.0/.
//
// Copyright (c) DUSK NETWORK. All rights reserved.

use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::net::SocketAddr;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use hyper::body::Body;
use rustls_pemfile::{certs, pkcs8_private_keys};
use tokio::io::{self, AsyncRead, AsyncWrite, ReadBuf};
use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};
use tokio_rustls::rustls::internal::msgs::codec::Codec;
use tokio_rustls::rustls::pki_types::{
    CertificateDer, PrivateKeyDer, PrivatePkcs8KeyDer,
};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::server::TlsStream;
use tokio_rustls::TlsAcceptor;

pub struct Listener {
    acceptor: Option<TlsAcceptor>,
    inner: TcpListener,
}

impl Listener {
    pub async fn bind<A: ToSocketAddrs>(addr: A) -> io::Result<Self> {
        Ok(Self {
            acceptor: None,
            inner: TcpListener::bind(addr).await?,
        })
    }

    pub async fn bind_tls<A, P1, P2>(
        addr: A,
        cert_and_key: (P1, P2),
    ) -> io::Result<Self>
    where
        A: ToSocketAddrs,
        P1: AsRef<Path>,
        P2: AsRef<Path>,
    {
        let (cert, key) = cert_and_key;

        let cert_file = File::open(cert)?;
        let key_file = File::open(key)?;

        let mut cert_reader = BufReader::new(cert_file);
        let mut key_reader = BufReader::new(key_file);

        let cert = certs(&mut cert_reader)
            .collect::<io::Result<Vec<CertificateDer>>>()?;
        let key = pkcs8_private_keys(&mut key_reader).next().ok_or(
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid private key"),
        )??;
        let key = PrivateKeyDer::Pkcs8(key);

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(cert, key)
            .map_err(|e| {
                io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("Invalid certificate/key: {e}"),
                )
            })?;

        Ok(Self {
            acceptor: Some(TlsAcceptor::from(Arc::new(config))),
            inner: TcpListener::bind(addr).await?,
        })
    }

    pub async fn accept(&self) -> io::Result<Stream> {
        let (stream, _) = self.inner.accept().await?;

        let stream = match &self.acceptor {
            None => Stream::Raw(stream),
            Some(acceptor) => {
                let stream = acceptor.accept(stream).await?;
                Stream::Tls(stream)
            }
        };

        Ok(stream)
    }

    pub fn local_addr(&self) -> io::Result<SocketAddr> {
        self.inner.local_addr()
    }
}

pub enum Stream {
    Raw(TcpStream),
    Tls(TlsStream<TcpStream>),
}

impl AsyncRead for Stream {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<io::Result<()>> {
        match &mut *self {
            Stream::Raw(stream) => Pin::new(stream).poll_read(cx, buf),
            Stream::Tls(stream) => Pin::new(stream).poll_read(cx, buf),
        }
    }
}

impl AsyncWrite for Stream {
    fn poll_write(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &[u8],
    ) -> Poll<io::Result<usize>> {
        match &mut *self {
            Stream::Raw(stream) => Pin::new(stream).poll_write(cx, buf),
            Stream::Tls(stream) => Pin::new(stream).poll_write(cx, buf),
        }
    }

    fn poll_flush(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        match &mut *self {
            Stream::Raw(stream) => Pin::new(stream).poll_flush(cx),
            Stream::Tls(stream) => Pin::new(stream).poll_flush(cx),
        }
    }

    fn poll_shutdown(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<io::Result<()>> {
        match &mut *self {
            Stream::Raw(stream) => Pin::new(stream).poll_shutdown(cx),
            Stream::Tls(stream) => Pin::new(stream).poll_shutdown(cx),
        }
    }
}
