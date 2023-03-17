use nix::sys::socket::*;
use nix::unistd::*;
use std::io::{self, Write};
use std::str::FromStr;
use nix::sys::epoll::*;
use std::os::unix::prelude::RawFd;
use std::mem::transmute;

fn sync_client() {
    let fd = socket(AddressFamily::Inet, SockType::Stream, SockFlag::empty(), None).unwrap();
    let addr = SockaddrIn::from_str("39.156.66.10:80").unwrap();
    connect(fd, &addr).unwrap();

    // send all
    {
        let input = b"GET / HTTP/1.0\r\nHost: www.baidu.com\r\n\r\n";
        let mut pos: usize = 0;
        while pos < input.len() {
            let n = write(fd, &input[pos .. input.len()]).unwrap();
            pos += n;
        }
    }

    // recv all
    loop {
        let mut buf = [0u8; 128];
        let n = read(fd, &mut buf).unwrap();
        if n == 0 {
            break;
        }
        io::stdout().write_all(&buf[0 .. n]).unwrap();
    }
    close(fd).unwrap();
}

struct Context {
    fd: RawFd,
    flags: EpollFlags,
    input: Vec<u8>,
    pos: usize
}

impl Context {
    fn new(fd: RawFd, input: Vec<u8>) -> Self {
        Context {
            fd,
            flags: EpollFlags::EPOLLOUT.union(EpollFlags::EPOLLIN),
            input,
            pos: 0
        }
    }
}

fn handle_event(epfd: RawFd, done: &mut bool, event: &EpollEvent) {
    println!("handle event");

    let ctx: &mut Context = unsafe { transmute(event.data()) };
    let old_flags = ctx.flags;

    let flags = &event.events();
    if flags.contains(EpollFlags::EPOLLOUT) {
        println!("OUT");
        let n = write(ctx.fd, &ctx.input[ctx.pos .. ctx.input.len()]).unwrap();
        ctx.pos += n;
        if ctx.pos >= ctx.input.len() {
            ctx.flags.remove(EpollFlags::EPOLLOUT);
        }
    }
    if flags.contains(EpollFlags::EPOLLIN) {
        println!("IN");
        let mut buf = [0u8; 128];
        let n = read(ctx.fd, &mut buf).unwrap();
        if n == 0 {
            ctx.flags.remove(EpollFlags::EPOLLIN);
        } else {
            io::stdout().write_all(&buf[0 .. n]).unwrap();
        }
    }
    if ctx.flags.is_empty() {
        epoll_ctl(epfd, EpollOp::EpollCtlDel, ctx.fd, None).unwrap();
        close(ctx.fd).unwrap();
        *done = true;
    } else if ctx.flags != old_flags {
        let mut event_new = EpollEvent::new(ctx.flags, unsafe { transmute(&*ctx) });
        epoll_ctl(epfd, EpollOp::EpollCtlMod, ctx.fd, Some(&mut event_new)).unwrap();
    }
}

fn async_client() {
    let epfd = epoll_create().unwrap();

    // add initial event
    let fd = socket(AddressFamily::Inet, SockType::Stream, SockFlag::empty(), None).unwrap();
    let addr = SockaddrIn::from_str("127.0.0.1:8000").unwrap();
    connect(fd, &addr).unwrap();

    let mut ctx = Context::new(fd, b"GET / HTTP/1.0\r\nHost: www.baidu.com\r\n\r\n".to_vec());
    let mut first_event = EpollEvent::new(ctx.flags, unsafe { transmute(&mut ctx) });
    epoll_ctl(epfd, EpollOp::EpollCtlAdd, fd, Some(&mut first_event)).unwrap();

    let mut done = false;
    while !done {
        let mut events = [EpollEvent::empty(); 16];
        let n = epoll_wait(epfd, &mut events, -1).unwrap();
        for i in 0..n {
            handle_event(epfd, &mut done, &mut events[i]);
        }
    }
}

fn main() {
    async_client();
}
