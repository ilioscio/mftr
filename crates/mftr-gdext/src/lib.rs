//! `mftr-gdext`: the Godot side of the MFTR client.
//!
//! Godot never decides gameplay outcomes (04 §1). This extension owns the UDP socket and the
//! [`ClientSession`] (clock sync, prediction, interpolation) and exposes render state plus
//! command entry points to GDScript.

use godot::prelude::*;
use mftr_client::{ClientSession, Phase};
use mftr_sim::Vec2;
use std::net::UdpSocket;
use std::time::Instant;

struct MftrExtension;

#[gdextension]
unsafe impl ExtensionLibrary for MftrExtension {}

/// Connection to an `mftr-server`. Add it to the scene, call `connect_to_server`, then read
/// positions every frame. All positions are in game units (1 u = 0.01 m in the client scene).
#[derive(GodotClass)]
#[class(base = Node)]
pub struct MatchClient {
    base: Base<Node>,
    session: ClientSession,
    socket: Option<UdpSocket>,
    clock: Instant,
    next_hello: f64,
    last_error: GString,
}

#[godot_api]
impl INode for MatchClient {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            session: ClientSession::new(),
            socket: None,
            clock: Instant::now(),
            next_hello: 0.0,
            last_error: GString::new(),
        }
    }

    fn process(&mut self, _delta: f64) {
        self.pump();
    }

    fn exit_tree(&mut self) {
        self.disconnect_from_server();
    }
}

#[godot_api]
impl MatchClient {
    /// Open a UDP socket to `address` ("host:port") and start the handshake.
    #[func]
    fn connect_to_server(&mut self, address: GString) -> bool {
        self.disconnect_from_server();
        let result = UdpSocket::bind("0.0.0.0:0").and_then(|s| {
            s.connect(address.to_string())?;
            s.set_nonblocking(true)?;
            Ok(s)
        });
        match result {
            Ok(s) => {
                self.socket = Some(s);
                self.session = ClientSession::new();
                self.next_hello = 0.0;
                self.last_error = GString::new();
                true
            }
            Err(e) => {
                self.last_error = GString::from(&e.to_string());
                false
            }
        }
    }

    #[func]
    fn disconnect_from_server(&mut self) {
        if let Some(s) = self.socket.take() {
            let _ = s.send(&self.session.bye_packet());
        }
    }

    /// "connecting", "joining" or "playing" (empty when disconnected).
    #[func]
    fn phase(&self) -> GString {
        if self.socket.is_none() {
            return GString::new();
        }
        GString::from(match self.session.phase() {
            Phase::Connecting => "connecting",
            Phase::Joining => "joining",
            Phase::Playing => "playing",
        })
    }

    #[func]
    fn last_error(&self) -> GString {
        self.last_error.clone()
    }

    /// Right-click move to a ground point, in game units. Predicted immediately.
    #[func]
    fn move_to(&mut self, target: Vector2) {
        let now = self.now();
        if self.session.move_to(Vec2::new(target.x, target.y), now).is_some() {
            self.send_input(now);
        }
    }

    #[func]
    fn stop(&mut self) {
        let now = self.now();
        if self.session.stop(now).is_some() {
            self.send_input(now);
        }
    }

    #[func]
    fn own_unit_id(&self) -> i64 {
        self.session.unit().0 as i64
    }

    /// Own champion on the input timeline (03a §3), including correction smoothing.
    #[func]
    fn own_position(&self) -> Vector2 {
        let p = self.session.own_render_position(self.now()).unwrap_or(Vec2::ZERO);
        Vector2::new(p.x, p.y)
    }

    /// Remote units interpolated on `T_interp`: `{ unit_id: Vector2 }`.
    #[func]
    fn remote_positions(&self) -> VarDictionary {
        let mut d = VarDictionary::new();
        for (id, p) in self.session.remote_render_positions(self.now()) {
            d.set(id.0 as i64, Vector2::new(p.x, p.y));
        }
        d
    }

    /// Net graph data (03 §14).
    #[func]
    fn net_stats(&self) -> VarDictionary {
        let s = &self.session;
        let st = &s.stats;
        let mut d = VarDictionary::new();
        d.set("rtt_ms", s.rtt() * 1e3);
        d.set("margin_ms", s.margin() * 1e3);
        d.set("interp_ms", s.interp_buffer() * 1e3);
        d.set("commands", st.commands_issued as i64);
        d.set("late", st.commands_late as i64);
        d.set("mismatches", st.mismatches as i64);
        d.set("corrections", st.corrections.len() as i64);
        d.set("last_correction", st.corrections.last().copied().unwrap_or(0.0));
        d.set("visible_correction", s.visible_correction());
        d.set("kb_up", st.bytes_up as f64 / 1024.0);
        d.set("kb_down", st.bytes_down as f64 / 1024.0);
        d
    }
}

impl MatchClient {
    fn now(&self) -> f64 {
        self.clock.elapsed().as_secs_f64()
    }

    fn send_input(&mut self, now: f64) {
        let packet = self.session.input_packet(now);
        if let Some(s) = &self.socket {
            let _ = s.send(&packet);
        }
    }

    /// Drain the socket, advance the session, and send whatever is due.
    fn pump(&mut self) {
        let Some(socket) = self.socket.as_ref() else { return };
        let mut buf = [0u8; 2048];
        let mut packets = Vec::new();
        loop {
            match socket.recv(&mut buf) {
                Ok(n) => packets.push(buf[..n].to_vec()),
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                // Windows reports ICMP "port unreachable" as a reset; the server may just not be up yet.
                Err(e) if e.kind() == std::io::ErrorKind::ConnectionReset => break,
                Err(e) => {
                    self.last_error = GString::from(&e.to_string());
                    break;
                }
            }
        }
        let now = self.now();
        for p in packets {
            self.session.handle_packet(&p, now);
        }
        self.session.update(now);
        match self.session.phase() {
            Phase::Connecting if now >= self.next_hello => {
                let hello = self.session.hello_packet(now);
                if let Some(s) = &self.socket {
                    let _ = s.send(&hello);
                }
                self.next_hello = now + 0.25;
            }
            Phase::Playing if self.session.should_send(now) => self.send_input(now),
            _ => {}
        }
    }
}
