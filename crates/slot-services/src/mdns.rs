//! `gravy.local`: a name for the GRAVY Manager, so a phone can open it without typing the IP.
//!
//! BaseOS's Avahi only publishes its own `<hostname>.local`, so while the Manager is open this
//! answers mDNS questions (UDP 5353, shared with Avahi through port reuse) for one more name
//! with the console's address. Before it answers it asks whether someone else has the name; a
//! second GRAVY on the network becomes `gravy-2.local`, and so on. Phones or routers that do
//! not do mDNS still have the IP and the QR code.
//!
//! Every packet comes from whoever is on the network: reads are bounds-checked and a packet
//! that does not parse is dropped.

use std::io;
use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4, UdpSocket};
use std::os::fd::FromRawFd;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

pub const NAME: &str = "gravy";
const MAX_SUFFIX: u32 = 9;
const PORT: u16 = 5353;
const GROUP: Ipv4Addr = Ipv4Addr::new(224, 0, 0, 251);
const TYPE_A: u16 = 1;
const TYPE_ANY: u16 = 255;
const CLASS_IN: u16 = 1;
const CACHE_FLUSH: u16 = 0x8000;
const TTL: u32 = 120;
const MAX_JUMPS: usize = 16;

/// `gravy.local`, then `gravy-2.local` ... `gravy-9.local`.
fn names() -> impl Iterator<Item = String> {
    std::iter::once(format!("{NAME}.local"))
        .chain((2..=MAX_SUFFIX).map(|n| format!("{NAME}-{n}.local")))
}

fn put_name(out: &mut Vec<u8>, name: &str) {
    for label in name.split('.').filter(|l| !l.is_empty()) {
        let b = &label.as_bytes()[..label.len().min(63)];
        out.push(b.len() as u8);
        out.extend_from_slice(b);
    }
    out.push(0);
}

fn u16_at(p: &[u8], at: usize) -> Option<u16> {
    Some(u16::from_be_bytes([*p.get(at)?, *p.get(at + 1)?]))
}

/// The name at `start`, lower case, and where the bytes after it begin.
fn read_name(p: &[u8], start: usize) -> Option<(String, usize)> {
    let (mut at, mut end, mut jumps) = (start, None, 0);
    let mut name = String::new();
    loop {
        let len = *p.get(at)? as usize;
        if len == 0 {
            return Some((name, end.unwrap_or(at + 1)));
        }
        if len & 0xC0 == 0xC0 {
            jumps += 1;
            if jumps > MAX_JUMPS {
                return None;
            }
            end.get_or_insert(at + 2);
            at = (u16_at(p, at)? & 0x3FFF) as usize;
            continue;
        }
        let label = p.get(at + 1..at + 1 + len)?;
        if !name.is_empty() {
            name.push('.');
        }
        name.push_str(&String::from_utf8_lossy(label).to_ascii_lowercase());
        if name.len() > 255 {
            return None;
        }
        at += 1 + len;
    }
}

/// The question names, and the answer names with their address, in a packet.
struct Parsed {
    response: bool,
    questions: Vec<(String, u16)>,
    answers: Vec<(String, Option<Ipv4Addr>)>,
}

fn parse(p: &[u8]) -> Option<Parsed> {
    let flags = u16_at(p, 2)?;
    let qd = u16_at(p, 4)?;
    let records = u16_at(p, 6)? as usize + u16_at(p, 8)? as usize + u16_at(p, 10)? as usize;
    let mut at = 12;
    let mut questions = Vec::new();
    for _ in 0..qd {
        let (name, next) = read_name(p, at)?;
        questions.push((name, u16_at(p, next)?));
        at = next + 4;
    }
    let mut answers = Vec::new();
    for _ in 0..records.min(64) {
        let (name, next) = read_name(p, at)?;
        let kind = u16_at(p, next)?;
        let len = u16_at(p, next + 8)? as usize;
        let data = p.get(next + 10..next + 10 + len)?;
        let addr =
            (kind == TYPE_A && len == 4).then(|| Ipv4Addr::new(data[0], data[1], data[2], data[3]));
        answers.push((name, addr));
        at = next + 10 + len;
    }
    Some(Parsed {
        response: flags & 0x8000 != 0,
        questions,
        answers,
    })
}

/// "Who has `name`?"
fn query(name: &str) -> Vec<u8> {
    let mut out = vec![0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0];
    put_name(&mut out, name);
    out.extend_from_slice(&TYPE_ANY.to_be_bytes());
    out.extend_from_slice(&CLASS_IN.to_be_bytes());
    out
}

/// "`name` is `ip`." `id` echoes a legacy (non-5353) asker's query id; `ttl` 0 says goodbye.
fn answer(name: &str, ip: Ipv4Addr, id: u16, ttl: u32) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&id.to_be_bytes());
    out.extend_from_slice(&[0x84, 0, 0, 0, 0, 1, 0, 0, 0, 0]);
    put_name(&mut out, name);
    out.extend_from_slice(&TYPE_A.to_be_bytes());
    out.extend_from_slice(&(CLASS_IN | CACHE_FLUSH).to_be_bytes());
    out.extend_from_slice(&ttl.to_be_bytes());
    out.extend_from_slice(&4u16.to_be_bytes());
    out.extend_from_slice(&ip.octets());
    out
}

/// Whether `p` asks for `name`'s address.
fn asks_for(p: &Parsed, name: &str) -> bool {
    !p.response
        && p.questions
            .iter()
            .any(|(n, kind)| n == name && (*kind == TYPE_A || *kind == TYPE_ANY))
}

/// Whether `p` says some other address has `name`, or asks for it while probing it too.
fn taken_by_other(p: &Parsed, name: &str, ip: Ipv4Addr) -> bool {
    p.answers
        .iter()
        .any(|(n, addr)| n == name && addr.is_some_and(|a| a != ip))
}

fn set<T>(fd: i32, level: i32, opt: i32, value: &T) -> io::Result<()> {
    let rc = unsafe {
        libc::setsockopt(
            fd,
            level,
            opt,
            value as *const T as *const libc::c_void,
            std::mem::size_of::<T>() as libc::socklen_t,
        )
    };
    if rc < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

/// UDP 5353, shared with Avahi, joined to the mDNS group on the interface that owns `ip`.
fn open(ip: Ipv4Addr) -> io::Result<UdpSocket> {
    let fd = unsafe { libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0) };
    if fd < 0 {
        return Err(io::Error::last_os_error());
    }
    let sock = unsafe { UdpSocket::from_raw_fd(fd) };
    let on: libc::c_int = 1;
    set(fd, libc::SOL_SOCKET, libc::SO_REUSEADDR, &on)?;
    set(fd, libc::SOL_SOCKET, libc::SO_REUSEPORT, &on)?;
    let bind = libc::sockaddr_in {
        sin_family: libc::AF_INET as libc::sa_family_t,
        sin_port: PORT.to_be(),
        sin_addr: libc::in_addr { s_addr: 0 },
        sin_zero: [0; 8],
    };
    let rc = unsafe {
        libc::bind(
            fd,
            &bind as *const libc::sockaddr_in as *const libc::sockaddr,
            std::mem::size_of::<libc::sockaddr_in>() as libc::socklen_t,
        )
    };
    if rc < 0 {
        return Err(io::Error::last_os_error());
    }
    let here = libc::in_addr {
        s_addr: u32::from(ip).to_be(),
    };
    let member = libc::ip_mreq {
        imr_multiaddr: libc::in_addr {
            s_addr: u32::from(GROUP).to_be(),
        },
        imr_interface: here,
    };
    set(fd, libc::IPPROTO_IP, libc::IP_ADD_MEMBERSHIP, &member)?;
    set(fd, libc::IPPROTO_IP, libc::IP_MULTICAST_IF, &here)?;
    let hops: libc::c_int = 255;
    set(fd, libc::IPPROTO_IP, libc::IP_MULTICAST_TTL, &hops)?;
    Ok(sock)
}

fn recv(sock: &UdpSocket, wait: Duration) -> Option<(Vec<u8>, SocketAddr)> {
    sock.set_read_timeout(Some(wait.max(Duration::from_millis(10))))
        .ok()?;
    let mut buf = [0u8; 1500];
    let (n, from) = sock.recv_from(&mut buf).ok()?;
    Some((buf[..n].to_vec(), from))
}

/// Asks three times, 250 ms apart, whether anyone has `name`, as RFC 6762 probing does.
fn free(sock: &UdpSocket, name: &str, ip: Ipv4Addr) -> bool {
    let group = SocketAddrV4::new(GROUP, PORT);
    for _ in 0..3 {
        let _ = sock.send_to(&query(name), group);
        let until = Instant::now() + Duration::from_millis(250);
        while let Some(left) = until.checked_duration_since(Instant::now()) {
            let Some((p, _)) = recv(sock, left) else {
                break;
            };
            if parse(&p).is_some_and(|p| taken_by_other(&p, name, ip)) {
                return false;
            }
        }
    }
    true
}

/// Answers for the Manager's name until dropped, then says goodbye.
pub struct Responder {
    pub name: String,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Responder {
    /// Claims the first free name and starts answering for it. None when the socket cannot be
    /// had or every name is taken; the Manager then only shows its IP.
    pub fn start(ip: Ipv4Addr) -> Option<Responder> {
        let sock = open(ip).ok()?;
        let name = names().find(|n| free(&sock, n, ip))?;
        let group = SocketAddrV4::new(GROUP, PORT);
        let _ = sock.send_to(&answer(&name, ip, 0, TTL), group);
        let stop = Arc::new(AtomicBool::new(false));
        let thread = {
            let (stop, name) = (stop.clone(), name.clone());
            std::thread::spawn(move || {
                while !stop.load(Ordering::SeqCst) {
                    let Some((p, from)) = recv(&sock, Duration::from_millis(200)) else {
                        continue;
                    };
                    let Some(parsed) = parse(&p) else {
                        continue;
                    };
                    if !asks_for(&parsed, &name) {
                        continue;
                    }
                    // A plain DNS asker (not from 5353) wants a unicast reply with its id.
                    match from {
                        SocketAddr::V4(f) if f.port() != PORT => {
                            let id = u16_at(&p, 0).unwrap_or(0);
                            let _ = sock.send_to(&answer(&name, ip, id, 10), f);
                        }
                        _ => {
                            let _ = sock.send_to(&answer(&name, ip, 0, TTL), group);
                        }
                    }
                }
                let _ = sock.send_to(&answer(&name, ip, 0, 0), group);
            })
        };
        Some(Responder {
            name,
            stop,
            thread: Some(thread),
        })
    }
}

impl Drop for Responder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_question_for_the_name_is_heard_in_any_case() {
        let mut q = query("Gravy.Local");
        let p = parse(&q).unwrap();
        assert!(asks_for(&p, "gravy.local"));
        assert!(!asks_for(&p, "gravy-2.local"));
        q[2] = 0x84;
        assert!(!asks_for(&parse(&q).unwrap(), "gravy.local"));
    }

    #[test]
    fn an_answer_round_trips_and_only_another_address_takes_the_name() {
        let ip = Ipv4Addr::new(192, 168, 1, 23);
        let p = parse(&answer("gravy.local", ip, 7, TTL)).unwrap();
        assert!(p.response);
        assert_eq!(p.answers, vec![("gravy.local".into(), Some(ip))]);
        assert!(!taken_by_other(&p, "gravy.local", ip));
        assert!(taken_by_other(
            &p,
            "gravy.local",
            Ipv4Addr::new(192, 168, 1, 24)
        ));
    }

    #[test]
    fn hostile_packets_are_dropped_not_followed() {
        // A name pointing at itself, and a packet cut short.
        let looped = [0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0xC0, 12, 0, 1, 0, 1];
        assert!(parse(&looped).is_none());
        assert!(parse(&query("gravy.local")[..15]).is_none());
    }

    #[test]
    fn names_count_up_from_plain_gravy() {
        let n: Vec<_> = names().collect();
        assert_eq!(n[0], "gravy.local");
        assert_eq!(n[1], "gravy-2.local");
        assert_eq!(n.len(), 9);
    }
}
