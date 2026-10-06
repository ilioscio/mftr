//! Secure transport (D40; 03b §1, §11): a thin encrypted layer under the packet layer.
//!
//! Sans-IO like the rest of the crate: [`SecureClient`] and [`SecureServer`] turn plaintext
//! game packets into datagrams and back, and the caller owns the socket and the clock. The
//! handshake is Noise XX, so both sides learn each other's static key: the server's is pinned
//! by the client (trust on first use, or a fingerprint in the address), the client's is its
//! identity (07 §3). Every datagram starts with a kind byte:
//!
//! ```text
//! client                                        server
//! HELLO    magic, protocol, cookie (none), e  →
//!                                             ←  COOKIE  stamp, MAC(address, stamp)   stateless
//! HELLO    magic, protocol, cookie, e         →
//!                                             ←  REPLY   Noise message 2 (e, ee, s, es)
//! CONFIRM  Noise message 3 (s, se)            →  sent with every DATA until the server answers
//! DATA     seq u16, ciphertext, tag           ↔
//! ```
//!
//! A HELLO is padded so that no unauthenticated reply is larger than the request (no
//! amplification), and the server spends no work and keeps no state for an address until that
//! address has echoed a cookie. DATA carries the low 16 bits of a 64-bit nonce, which the
//! receiver extends from the highest one it has seen; a 64-packet window drops replays.

use blake2::{Blake2s256, Digest};
use snow::{Builder, HandshakeState, StatelessTransportState};
use std::collections::HashMap;
use std::fmt;
use std::net::SocketAddr;
use std::path::Path;
use std::str::FromStr;

const NOISE_PARAMS: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2s";
const MAGIC: [u8; 4] = *b"MFTR";

const HELLO: u8 = 1;
const COOKIE: u8 = 2;
const REPLY: u8 = 3;
const CONFIRM: u8 = 4;
const DATA: u8 = 5;
/// Server → client, plaintext: "I speak protocol N" (the client's HELLO had another).
const VERSION: u8 = 6;

const TAG: usize = 16;
const COOKIE_LEN: usize = 4 + 16;
/// Noise message 1 is the client's ephemeral key.
const E_LEN: usize = 32;
/// Every HELLO is padded to this size: larger than any reply to it.
const HELLO_LEN: usize = 160;
/// Bytes a DATA packet adds to the game packet it carries.
pub const OVERHEAD: usize = 1 + 2 + TAG;

/// A cookie is valid in the window it was issued in and the next one.
const COOKIE_SECONDS: f64 = 10.0;
/// Half-done handshakes are forgotten after this long.
const HANDSHAKE_TIMEOUT: f64 = 10.0;
/// Sessions silent this long are closed (the game layer gives up after 10 s).
pub const SESSION_TIMEOUT: f64 = 30.0;
const MAX_PENDING: usize = 256;
const MAX_SESSIONS: usize = 256;

/// Binds the handshake to the protocol version: peers on different versions can't connect.
const PROLOGUE: [u8; 6] = {
    let v = crate::PROTOCOL_VERSION.to_le_bytes();
    [MAGIC[0], MAGIC[1], MAGIC[2], MAGIC[3], v[0], v[1]]
};

fn builder() -> Builder<'static> {
    Builder::new(NOISE_PARAMS.parse().expect("valid Noise parameters"))
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn unhex<const N: usize>(s: &str) -> Option<[u8; N]> {
    let s = s.trim();
    if s.len() != N * 2 || !s.is_ascii() {
        return None;
    }
    let mut out = [0u8; N];
    for (i, o) in out.iter_mut().enumerate() {
        *o = u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).ok()?;
    }
    Some(out)
}

// ---- keys --------------------------------------------------------------------------------

/// A static X25519 key pair: a server's key, or a player's identity.
#[derive(Clone)]
pub struct Identity {
    private: [u8; 32],
    public: [u8; 32],
}

impl fmt::Debug for Identity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Identity({})", self.fingerprint())
    }
}

impl Identity {
    pub fn generate() -> Self {
        let mut private = [0u8; 32];
        getrandom::fill(&mut private).expect("the OS has no randomness");
        Self::from_private(private)
    }

    pub fn from_private(private: [u8; 32]) -> Self {
        let public = curve25519_dalek::MontgomeryPoint::mul_base_clamped(private).to_bytes();
        Self { private, public }
    }

    pub fn public(&self) -> [u8; 32] {
        self.public
    }

    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::of(&self.public)
    }

    /// The key file format: comment lines, then the private key in hex.
    pub fn to_text(&self, what: &str) -> String {
        format!("# MFTR {what} (keep it secret). Fingerprint: {}\n{}\n", self.fingerprint(), hex(&self.private))
    }

    pub fn from_text(text: &str) -> Option<Self> {
        let line = text.lines().map(str::trim).find(|l| !l.is_empty() && !l.starts_with('#'))?;
        unhex::<32>(line).map(Self::from_private)
    }

    /// Read the key at `path`, or create one there (readable by the owner only). Returns the
    /// key and whether it was created.
    pub fn load_or_create(path: &Path, what: &str) -> std::io::Result<(Self, bool)> {
        match std::fs::read_to_string(path) {
            Ok(text) => Self::from_text(&text).map(|k| (k, false)).ok_or_else(|| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, format!("{} is not an MFTR key", path.display()))
            }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let key = Self::generate();
                if let Some(dir) = path.parent().filter(|d| !d.as_os_str().is_empty()) {
                    std::fs::create_dir_all(dir)?;
                }
                let mut options = std::fs::OpenOptions::new();
                options.write(true).create_new(true);
                #[cfg(unix)]
                std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
                std::io::Write::write_all(&mut options.open(path)?, key.to_text(what).as_bytes())?;
                Ok((key, true))
            }
            Err(e) => Err(e),
        }
    }
}

/// A short, shareable name for a public key: 128 bits of BLAKE2s of it, in hex.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint(pub [u8; 16]);

impl Fingerprint {
    pub fn of(public: &[u8]) -> Self {
        let digest = Blake2s256::new().chain_update(b"mftr key").chain_update(public).finalize();
        let mut fp = [0u8; 16];
        fp.copy_from_slice(&digest[..16]);
        Fingerprint(fp)
    }
}

impl fmt::Display for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&hex(&self.0))
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Fingerprint({self})")
    }
}

impl FromStr for Fingerprint {
    type Err = String;
    fn from_str(s: &str) -> Result<Self, String> {
        unhex::<16>(s).map(Fingerprint).ok_or_else(|| format!("{s:?} is not a key fingerprint (32 hex digits)"))
    }
}

/// Split a server address `host:port[#fingerprint]` into the socket address and the pinned key.
pub fn parse_address(address: &str) -> Result<(&str, Option<Fingerprint>), String> {
    match address.trim().split_once('#') {
        Some((host, fp)) => Ok((host, Some(fp.parse()?))),
        None => Ok((address.trim(), None)),
    }
}

/// Servers a client has met (trust on first use): one `address fingerprint` per line.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct KnownServers {
    entries: Vec<(String, Fingerprint)>,
}

impl KnownServers {
    pub fn from_text(text: &str) -> Self {
        let entries = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| {
                let (addr, fp) = l.split_once(char::is_whitespace)?;
                Some((addr.to_string(), fp.trim().parse().ok()?))
            })
            .collect();
        Self { entries }
    }

    pub fn to_text(&self) -> String {
        let mut s = String::from("# MFTR servers this client trusts: address and key fingerprint.\n");
        for (addr, fp) in &self.entries {
            s.push_str(&format!("{addr} {fp}\n"));
        }
        s
    }

    pub fn get(&self, address: &str) -> Option<Fingerprint> {
        self.entries.iter().find(|(a, _)| a == address).map(|(_, fp)| *fp)
    }

    pub fn set(&mut self, address: &str, fp: Fingerprint) {
        match self.entries.iter_mut().find(|(a, _)| a == address) {
            Some(e) => e.1 = fp,
            None => self.entries.push((address.to_string(), fp)),
        }
    }
}

// ---- the encrypted channel ---------------------------------------------------------------

/// Which 64-bit nonces have arrived: the highest, and a bitmap of the 64 before it.
#[derive(Clone, Debug, Default)]
struct ReplayWindow {
    highest: Option<u64>,
    /// Bit `i` set = nonce `highest - 1 - i` arrived.
    bits: u64,
}

impl ReplayWindow {
    /// The full nonce whose low 16 bits are `seq`, closest to the highest seen.
    fn expand(&self, seq: u16) -> u64 {
        let Some(h) = self.highest else { return seq as u64 };
        let c = (h & !0xffff) | seq as u64;
        [c.checked_sub(0x1_0000), Some(c), c.checked_add(0x1_0000)]
            .into_iter()
            .flatten()
            .min_by_key(|n| n.abs_diff(h))
            .unwrap_or(c)
    }

    fn is_fresh(&self, n: u64) -> bool {
        match self.highest {
            None => true,
            Some(h) if n > h => true,
            Some(h) if n == h => false,
            Some(h) => {
                let back = h - n;
                back <= 64 && self.bits & (1 << (back - 1)) == 0
            }
        }
    }

    fn accept(&mut self, n: u64) {
        match self.highest {
            Some(h) if n <= h => {
                if h - n <= 64 {
                    self.bits |= 1 << (h - n - 1);
                }
            }
            Some(h) => {
                let shift = n - h;
                self.bits =
                    if shift > 64 { 0 } else { ((self.bits << 1) | 1).checked_shl(shift as u32 - 1).unwrap_or(0) };
                self.highest = Some(n);
            }
            None => self.highest = Some(n),
        }
    }
}

struct Channel {
    transport: StatelessTransportState,
    next_nonce: u64,
    replay: ReplayWindow,
}

impl Channel {
    fn new(transport: StatelessTransportState) -> Self {
        Self { transport, next_nonce: 0, replay: ReplayWindow::default() }
    }

    fn seal(&mut self, payload: &[u8]) -> Vec<u8> {
        let n = self.next_nonce;
        self.next_nonce += 1;
        let mut out = vec![0u8; 3 + payload.len() + TAG];
        out[0] = DATA;
        out[1..3].copy_from_slice(&(n as u16).to_le_bytes());
        let len = self.transport.write_message(n, payload, &mut out[3..]).expect("game packets fit a Noise message");
        out.truncate(3 + len);
        out
    }

    fn open(&mut self, datagram: &[u8]) -> Option<Vec<u8>> {
        if datagram.len() < OVERHEAD || datagram[0] != DATA {
            return None;
        }
        let n = self.replay.expand(u16::from_le_bytes([datagram[1], datagram[2]]));
        if !self.replay.is_fresh(n) {
            return None;
        }
        let mut out = vec![0u8; datagram.len()];
        let len = self.transport.read_message(n, &datagram[3..], &mut out).ok()?;
        out.truncate(len);
        self.replay.accept(n);
        Some(out)
    }
}

// ---- client ------------------------------------------------------------------------------

/// Why a connection can't be made.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SecureError {
    /// The server speaks another protocol version.
    Version { server: u16 },
    /// The server's key isn't the pinned one.
    KeyChanged { expected: Fingerprint, got: Fingerprint },
}

impl fmt::Display for SecureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SecureError::Version { server } => write!(
                f,
                "version mismatch: the server speaks protocol {server}, this client {}",
                crate::PROTOCOL_VERSION
            ),
            SecureError::KeyChanged { expected, got } => write!(
                f,
                "the server's key changed (expected {expected}, got {got}); \
                 if its operator replaced the key, connect with address#{got}"
            ),
        }
    }
}

enum ClientState {
    Hello { hs: Box<HandshakeState>, e: [u8; E_LEN] },
    Open { channel: Channel, confirm: Vec<u8>, confirmed: bool },
    Failed,
}

/// One connection to a server.
pub struct SecureClient {
    state: ClientState,
    expected: Option<Fingerprint>,
    server: Option<Fingerprint>,
    cookie: [u8; COOKIE_LEN],
    error: Option<SecureError>,
    outgoing: Vec<Vec<u8>>,
}

impl SecureClient {
    /// Connect as `identity`; with `expected`, only to a server with that key.
    pub fn new(identity: &Identity, expected: Option<Fingerprint>) -> Self {
        let mut hs = builder()
            .local_private_key(&identity.private)
            .and_then(|b| b.prologue(&PROLOGUE))
            .and_then(|b| b.build_initiator())
            .expect("a valid initiator");
        let mut msg1 = [0u8; 64];
        let len = hs.write_message(&[], &mut msg1).expect("Noise message 1");
        assert_eq!(len, E_LEN);
        let mut e = [0u8; E_LEN];
        e.copy_from_slice(&msg1[..E_LEN]);
        Self {
            state: ClientState::Hello { hs: Box::new(hs), e },
            expected,
            server: None,
            cookie: [0; COOKIE_LEN],
            error: None,
            outgoing: Vec::new(),
        }
    }

    fn hello(&self, e: &[u8; E_LEN]) -> Vec<u8> {
        let mut p = Vec::with_capacity(HELLO_LEN);
        p.push(HELLO);
        p.extend_from_slice(&MAGIC);
        p.extend_from_slice(&crate::PROTOCOL_VERSION.to_le_bytes());
        p.extend_from_slice(&self.cookie);
        p.extend_from_slice(e);
        p.resize(HELLO_LEN, 0);
        p
    }

    /// Datagrams that carry `payload` (a game packet). Until the handshake is done they are
    /// handshake packets instead, and the payload is dropped: the game layer repeats its
    /// hello, which retransmits the handshake too.
    pub fn seal(&mut self, payload: &[u8]) -> Vec<Vec<u8>> {
        match &mut self.state {
            ClientState::Hello { e, .. } => {
                let e = *e;
                vec![self.hello(&e)]
            }
            ClientState::Open { channel, confirm, confirmed } => {
                let data = channel.seal(payload);
                if *confirmed { vec![data] } else { vec![confirm.clone(), data] }
            }
            ClientState::Failed => Vec::new(),
        }
    }

    /// Handle a datagram from the server. Returns the game packet it carried, if any; check
    /// [`Self::take_outgoing`] afterwards for handshake replies to send.
    pub fn receive(&mut self, datagram: &[u8]) -> Option<Vec<u8>> {
        let (&kind, body) = datagram.split_first()?;
        match (&mut self.state, kind) {
            (ClientState::Hello { e, .. }, COOKIE) if body.len() == COOKIE_LEN => {
                let e = *e;
                self.cookie.copy_from_slice(body);
                let hello = self.hello(&e);
                self.outgoing.push(hello);
                None
            }
            (ClientState::Hello { .. }, VERSION) if body.len() == 2 => {
                self.fail(SecureError::Version { server: u16::from_le_bytes([body[0], body[1]]) });
                None
            }
            (ClientState::Hello { hs, .. }, REPLY) => {
                let mut scratch = [0u8; 128];
                // A forged reply fails here and is ignored.
                hs.read_message(body, &mut scratch).ok()?;
                let got = Fingerprint::of(hs.get_remote_static()?);
                self.server = Some(got);
                if let Some(expected) = self.expected
                    && expected != got
                {
                    self.fail(SecureError::KeyChanged { expected, got });
                    return None;
                }
                let mut msg3 = [0u8; 128];
                let len = hs.write_message(&[], &mut msg3).ok()?;
                let mut confirm = vec![CONFIRM];
                confirm.extend_from_slice(&msg3[..len]);
                let ClientState::Hello { hs, .. } = std::mem::replace(&mut self.state, ClientState::Failed) else {
                    unreachable!()
                };
                let transport = hs.into_stateless_transport_mode().ok()?;
                self.outgoing.push(confirm.clone());
                self.state = ClientState::Open { channel: Channel::new(transport), confirm, confirmed: false };
                None
            }
            (ClientState::Open { channel, confirmed, .. }, DATA) => {
                let payload = channel.open(datagram)?;
                *confirmed = true;
                Some(payload)
            }
            _ => None,
        }
    }

    fn fail(&mut self, e: SecureError) {
        self.error = Some(e);
        self.state = ClientState::Failed;
    }

    /// Handshake packets to send right away (in reply to a cookie or the server's reply).
    pub fn take_outgoing(&mut self) -> Vec<Vec<u8>> {
        std::mem::take(&mut self.outgoing)
    }

    /// The handshake is done: game packets go through.
    pub fn is_open(&self) -> bool {
        matches!(self.state, ClientState::Open { .. })
    }

    pub fn error(&self) -> Option<&SecureError> {
        self.error.as_ref()
    }

    /// The server's key, once its reply arrived.
    pub fn server_fingerprint(&self) -> Option<Fingerprint> {
        self.server
    }
}

// ---- server ------------------------------------------------------------------------------

/// What a datagram meant to the server.
#[derive(Debug, PartialEq, Eq)]
pub enum Received {
    Nothing,
    /// Send this back to the sender (a handshake step).
    Reply(Vec<u8>),
    /// A new session from this address (it replaces any earlier one), with the client's key.
    Connected {
        client: [u8; 32],
    },
    /// A game packet.
    Data(Vec<u8>),
}

#[derive(Clone, Debug, Default)]
pub struct SecureStats {
    pub cookies: u64,
    pub handshakes: u64,
    pub sessions: u64,
    /// Datagrams dropped: malformed, unauthenticated, replayed, or over a limit.
    pub dropped: u64,
}

struct Pending {
    hs: Box<HandshakeState>,
    e: [u8; E_LEN],
    reply: Vec<u8>,
    since: f64,
}

struct Session {
    channel: Channel,
    last_heard: f64,
}

/// The server side: sessions by client address.
pub struct SecureServer {
    identity: Identity,
    secret: [u8; 32],
    pending: HashMap<SocketAddr, Pending>,
    sessions: HashMap<SocketAddr, Session>,
    pub stats: SecureStats,
}

impl SecureServer {
    pub fn new(identity: Identity) -> Self {
        let mut secret = [0u8; 32];
        getrandom::fill(&mut secret).expect("the OS has no randomness");
        Self { identity, secret, pending: HashMap::new(), sessions: HashMap::new(), stats: SecureStats::default() }
    }

    pub fn fingerprint(&self) -> Fingerprint {
        self.identity.fingerprint()
    }

    fn cookie_mac(&self, from: SocketAddr, stamp: u32) -> [u8; 16] {
        let digest = Blake2s256::new()
            .chain_update(self.secret)
            .chain_update(from.to_string())
            .chain_update(stamp.to_le_bytes())
            .finalize();
        let mut mac = [0u8; 16];
        mac.copy_from_slice(&digest[..16]);
        mac
    }

    fn cookie(&self, from: SocketAddr, now: f64) -> Vec<u8> {
        let stamp = (now / COOKIE_SECONDS) as u32;
        let mut p = vec![COOKIE];
        p.extend_from_slice(&stamp.to_le_bytes());
        p.extend_from_slice(&self.cookie_mac(from, stamp));
        p
    }

    fn cookie_valid(&self, from: SocketAddr, cookie: &[u8], now: f64) -> bool {
        let stamp = u32::from_le_bytes([cookie[0], cookie[1], cookie[2], cookie[3]]);
        let current = (now / COOKIE_SECONDS) as u32;
        let mac = self.cookie_mac(from, stamp);
        // Constant time, so the MAC can't be guessed byte by byte.
        let diff = mac.iter().zip(&cookie[4..]).fold(0u8, |d, (a, b)| d | (a ^ b));
        (stamp == current || stamp.wrapping_add(1) == current) && diff == 0
    }

    /// Handle one datagram from `from`.
    pub fn receive(&mut self, from: SocketAddr, datagram: &[u8], now: f64) -> Received {
        let r = self.receive_inner(from, datagram, now);
        if r == Received::Nothing {
            self.stats.dropped += 1;
        }
        r
    }

    fn receive_inner(&mut self, from: SocketAddr, datagram: &[u8], now: f64) -> Received {
        let Some((&kind, body)) = datagram.split_first() else { return Received::Nothing };
        match kind {
            HELLO => {
                // Unpadded hellos would let a spoofed sender get more back than it sent.
                if datagram.len() < HELLO_LEN || body[..4] != MAGIC {
                    return Received::Nothing;
                }
                let version = u16::from_le_bytes([body[4], body[5]]);
                if version != crate::PROTOCOL_VERSION {
                    let mut p = vec![VERSION];
                    p.extend_from_slice(&crate::PROTOCOL_VERSION.to_le_bytes());
                    return Received::Reply(p);
                }
                let cookie = &body[6..6 + COOKIE_LEN];
                if !self.cookie_valid(from, cookie, now) {
                    self.stats.cookies += 1;
                    return Received::Reply(self.cookie(from, now));
                }
                let mut e = [0u8; E_LEN];
                e.copy_from_slice(&body[6 + COOKIE_LEN..6 + COOKIE_LEN + E_LEN]);
                if let Some(p) = self.pending.get(&from)
                    && p.e == e
                {
                    return Received::Reply(p.reply.clone()); // our reply was lost
                }
                if self.pending.len() >= MAX_PENDING {
                    return Received::Nothing;
                }
                let Ok(mut hs) = builder()
                    .local_private_key(&self.identity.private)
                    .and_then(|b| b.prologue(&PROLOGUE))
                    .and_then(|b| b.build_responder())
                else {
                    return Received::Nothing;
                };
                let mut scratch = [0u8; 64];
                if hs.read_message(&e, &mut scratch).is_err() {
                    return Received::Nothing;
                }
                let mut msg2 = [0u8; 128];
                let Ok(len) = hs.write_message(&[], &mut msg2) else { return Received::Nothing };
                let mut reply = vec![REPLY];
                reply.extend_from_slice(&msg2[..len]);
                self.stats.handshakes += 1;
                self.pending.insert(from, Pending { hs: Box::new(hs), e, reply: reply.clone(), since: now });
                Received::Reply(reply)
            }
            CONFIRM => {
                if !self.sessions.contains_key(&from) && self.sessions.len() >= MAX_SESSIONS {
                    return Received::Nothing;
                }
                let Some(mut p) = self.pending.remove(&from) else { return Received::Nothing };
                let mut scratch = [0u8; 64];
                if p.hs.read_message(body, &mut scratch).is_err() {
                    // Not from our client (or garbled): keep waiting for the real one.
                    self.pending.insert(from, p);
                    return Received::Nothing;
                }
                let Some(client) = p.hs.get_remote_static().and_then(|k| <[u8; 32]>::try_from(k).ok()) else {
                    return Received::Nothing;
                };
                let Ok(transport) = p.hs.into_stateless_transport_mode() else { return Received::Nothing };
                self.stats.sessions += 1;
                self.sessions.insert(from, Session { channel: Channel::new(transport), last_heard: now });
                Received::Connected { client }
            }
            DATA => {
                let Some(s) = self.sessions.get_mut(&from) else { return Received::Nothing };
                match s.channel.open(datagram) {
                    Some(payload) => {
                        s.last_heard = now;
                        Received::Data(payload)
                    }
                    None => Received::Nothing,
                }
            }
            _ => Received::Nothing,
        }
    }

    /// The datagram carrying `payload` to `to`, if it has a session.
    pub fn seal(&mut self, to: SocketAddr, payload: &[u8]) -> Option<Vec<u8>> {
        self.sessions.get_mut(&to).map(|s| s.channel.seal(payload))
    }

    /// Forget stale handshakes and close silent sessions. Returns the closed sessions.
    pub fn expire(&mut self, now: f64) -> Vec<SocketAddr> {
        self.pending.retain(|_, p| now - p.since < HANDSHAKE_TIMEOUT);
        let gone: Vec<SocketAddr> =
            self.sessions.iter().filter(|(_, s)| now - s.last_heard >= SESSION_TIMEOUT).map(|(a, _)| *a).collect();
        for a in &gone {
            self.sessions.remove(a);
        }
        gone
    }

    pub fn session_count(&self) -> usize {
        self.sessions.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn addr(port: u16) -> SocketAddr {
        SocketAddr::from(([10, 0, 0, 1], port))
    }

    /// Run the handshake to completion (no loss). Returns the client key the server saw.
    fn connect(client: &mut SecureClient, server: &mut SecureServer, from: SocketAddr, now: f64) -> [u8; 32] {
        let mut to_server = client.seal(b"hello");
        for _ in 0..8 {
            let mut to_client = Vec::new();
            for d in to_server.drain(..) {
                match server.receive(from, &d, now) {
                    Received::Reply(r) => to_client.push(r),
                    Received::Connected { client } => return client,
                    _ => {}
                }
            }
            for d in to_client {
                client.receive(&d);
                to_server.extend(client.take_outgoing());
            }
        }
        panic!("no session");
    }

    #[test]
    fn handshake_cookie_and_data_both_ways() {
        let (me, srv) = (Identity::generate(), Identity::generate());
        let mut server = SecureServer::new(srv.clone());
        let mut client = SecureClient::new(&me, Some(srv.fingerprint()));
        // The first hello only gets a cookie; no state is kept for the address.
        let hello = client.seal(b"x").remove(0);
        assert_eq!(hello.len(), HELLO_LEN);
        let Received::Reply(cookie) = server.receive(addr(1), &hello, 0.0) else { panic!() };
        assert!(cookie.len() < hello.len());
        assert!(server.pending.is_empty());

        let seen = connect(&mut client, &mut server, addr(1), 0.0);
        assert_eq!(seen, me.public(), "the server learns who connected");
        assert!(client.is_open());
        assert_eq!(client.server_fingerprint(), Some(srv.fingerprint()));

        // Until the server answers, every data packet rides with the confirm.
        let up = client.seal(b"input");
        assert_eq!(up.len(), 2);
        assert_eq!(server.receive(addr(1), &up[0], 0.1), Received::Nothing, "a repeated confirm is ignored");
        assert_eq!(server.receive(addr(1), &up[1], 0.1), Received::Data(b"input".to_vec()));
        let down = server.seal(addr(1), b"snapshot").unwrap();
        assert_eq!(down.len(), b"snapshot".len() + OVERHEAD);
        assert_eq!(client.receive(&down), Some(b"snapshot".to_vec()));
        assert_eq!(client.seal(b"more").len(), 1, "confirmed");
        assert!(server.seal(addr(2), b"x").is_none(), "no session, nothing sent");
    }

    #[test]
    fn replays_tampering_and_reordering() {
        let srv = Identity::generate();
        let mut server = SecureServer::new(srv);
        let mut client = SecureClient::new(&Identity::generate(), None);
        connect(&mut client, &mut server, addr(1), 0.0);
        let packets: Vec<Vec<u8>> = (0..5u8).map(|i| server.seal(addr(1), &[i]).unwrap()).collect();
        assert_eq!(client.receive(&packets[3]), Some(vec![3]));
        assert_eq!(client.receive(&packets[1]), Some(vec![1]), "late but in the window");
        assert_eq!(client.receive(&packets[3]), None, "replayed");
        assert_eq!(client.receive(&packets[1]), None, "replayed");
        let mut bad = packets[4].clone();
        bad[5] ^= 1;
        assert_eq!(client.receive(&bad), None, "tampered");
        assert_eq!(client.receive(&packets[4]), Some(vec![4]), "the real one still counts");
    }

    #[test]
    fn a_pinned_key_that_differs_is_refused() {
        let (srv, other) = (Identity::generate(), Identity::generate());
        let mut server = SecureServer::new(srv.clone());
        let mut client = SecureClient::new(&Identity::generate(), Some(other.fingerprint()));
        let mut d = client.seal(b"");
        for _ in 0..3 {
            let Received::Reply(r) = server.receive(addr(1), &d[0], 0.0) else { break };
            client.receive(&r);
            d = client.take_outgoing();
            if d.is_empty() {
                break;
            }
        }
        assert_eq!(
            client.error(),
            Some(&SecureError::KeyChanged { expected: other.fingerprint(), got: srv.fingerprint() })
        );
        assert!(client.seal(b"x").is_empty());
    }

    #[test]
    fn another_protocol_version_gets_a_clear_answer() {
        let mut server = SecureServer::new(Identity::generate());
        let mut client = SecureClient::new(&Identity::generate(), None);
        let mut hello = client.seal(b"").remove(0);
        hello[5..7].copy_from_slice(&(crate::PROTOCOL_VERSION - 1).to_le_bytes());
        let Received::Reply(r) = server.receive(addr(1), &hello, 0.0) else { panic!() };
        // The server's answer, as an older client would see it: reuse ours to read it.
        client.receive(&r);
        assert_eq!(client.error(), Some(&SecureError::Version { server: crate::PROTOCOL_VERSION }));
    }

    #[test]
    fn lost_handshake_packets_are_repeated() {
        let mut server = SecureServer::new(Identity::generate());
        let mut client = SecureClient::new(&Identity::generate(), None);
        let Received::Reply(cookie) = server.receive(addr(1), &client.seal(b"")[0], 0.0) else { panic!() };
        client.receive(&cookie);
        let hello = client.take_outgoing().remove(0);
        let Received::Reply(reply) = server.receive(addr(1), &hello, 0.0) else { panic!() };
        // The reply is lost; the game layer's next hello repeats the handshake hello, and the
        // server repeats the same reply without redoing the work.
        let again = client.seal(b"hello").remove(0);
        assert_eq!(server.receive(addr(1), &again, 0.3), Received::Reply(reply.clone()));
        assert_eq!(server.stats.handshakes, 1);
        client.receive(&reply);
        let _lost_confirm = client.take_outgoing();
        // The confirm is lost too: it rides along with the next data packet.
        let up = client.seal(b"hello");
        assert!(matches!(server.receive(addr(1), &up[0], 0.6), Received::Connected { .. }));
        assert_eq!(server.receive(addr(1), &up[1], 0.6), Received::Data(b"hello".to_vec()));
    }

    #[test]
    fn cookies_are_bound_to_the_address_and_expire() {
        let mut server = SecureServer::new(Identity::generate());
        let mut client = SecureClient::new(&Identity::generate(), None);
        let Received::Reply(cookie) = server.receive(addr(1), &client.seal(b"")[0], 0.0) else { panic!() };
        client.receive(&cookie);
        let hello = client.take_outgoing().remove(0);
        assert!(matches!(server.receive(addr(2), &hello, 0.0), Received::Reply(r) if r[0] == COOKIE));
        assert!(matches!(server.receive(addr(1), &hello, 25.0), Received::Reply(r) if r[0] == COOKIE));
        assert!(matches!(server.receive(addr(1), &hello, 15.0), Received::Reply(r) if r[0] == REPLY));
        // Unpadded hellos are dropped.
        assert_eq!(server.receive(addr(1), &hello[..100], 15.0), Received::Nothing);
    }

    #[test]
    fn a_restarted_client_on_the_same_address_gets_a_new_session() {
        let mut server = SecureServer::new(Identity::generate());
        let (a, b) = (Identity::generate(), Identity::generate());
        let mut first = SecureClient::new(&a, None);
        assert_eq!(connect(&mut first, &mut server, addr(1), 0.0), a.public());
        let mut second = SecureClient::new(&b, None);
        assert_eq!(connect(&mut second, &mut server, addr(1), 1.0), b.public());
        assert_eq!(server.session_count(), 1);
        let down = server.seal(addr(1), b"to b").unwrap();
        assert_eq!(first.receive(&down), None);
        assert_eq!(second.receive(&down), Some(b"to b".to_vec()));
        assert_eq!(server.expire(1.0 + SESSION_TIMEOUT), vec![addr(1)]);
    }

    #[test]
    fn nonces_extend_across_the_16_bit_wrap() {
        let mut w = ReplayWindow::default();
        for n in 0..70_000u64 {
            let e = w.expand(n as u16);
            assert_eq!(e, n);
            assert!(w.is_fresh(e));
            w.accept(e);
        }
        // A late packet from just before the wrap, and one too old to tell.
        assert_eq!(w.expand(65_535), 65_535);
        assert!(!w.is_fresh(69_999));
        let mut w = ReplayWindow::default();
        w.accept(100);
        w.accept(98);
        assert!(!w.is_fresh(98) && w.is_fresh(99) && !w.is_fresh(30));
        w.accept(300);
        assert!(!w.is_fresh(200) && w.is_fresh(299));
    }

    #[test]
    fn keys_fingerprints_addresses_and_known_servers() {
        let k = Identity::generate();
        let back = Identity::from_text(&k.to_text("server key")).unwrap();
        assert_eq!(back.public(), k.public());
        let fp = k.fingerprint();
        assert_eq!(fp.to_string().len(), 32);
        assert_eq!(fp.to_string().parse::<Fingerprint>(), Ok(fp));
        assert_eq!(parse_address("example.org:7777"), Ok(("example.org:7777", None)));
        assert_eq!(parse_address(&format!("1.2.3.4:7777#{fp}")), Ok(("1.2.3.4:7777", Some(fp))));
        assert!(parse_address("1.2.3.4:7777#nope").is_err());

        let mut known = KnownServers::default();
        known.set("a:1", fp);
        known.set("b:2", Identity::generate().fingerprint());
        known.set("a:1", Fingerprint([7; 16]));
        let back = KnownServers::from_text(&known.to_text());
        assert_eq!(back, known);
        assert_eq!(back.get("a:1"), Some(Fingerprint([7; 16])));
        assert_eq!(back.get("c:3"), None);

        let dir = std::env::temp_dir().join(format!("mftr-key-test-{}", std::process::id()));
        let path = dir.join("server.key");
        let (made, created) = Identity::load_or_create(&path, "server key").unwrap();
        let (loaded, again) = Identity::load_or_create(&path, "server key").unwrap();
        assert!(created && !again);
        assert_eq!(made.public(), loaded.public());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(&path).unwrap().permissions().mode() & 0o777, 0o600);
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
