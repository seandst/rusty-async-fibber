use std::io;
use std::io::prelude::*;
use std::net::TcpListener;
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod async_fibber;

struct ThreadPhone<T> {
    tx: mpsc::SyncSender<T>,
    n: usize,
}

fn main() -> io::Result<()> {
    let (tx, rx) = mpsc::sync_channel(10);
    let mut cache = async_fibber::fib_cache();

    // hand over the cache and receiver to a single worker thread
    // connections are concurrent, but generating and caching fib values isn't
    thread::spawn(move|| {
        for tp in rx.iter() {
            let tp = tp as ThreadPhone<_>;
            tp.tx.send(async_fibber::fib(tp.n, &mut cache)).unwrap();
        }
    });

    let addr = "0.0.0.0:1234";
    let listener = TcpListener::bind(addr)?;
    println!("Listening at {}", addr);
    Ok(for stream in listener.incoming() {
        let inner_tx = tx.clone();
        thread::spawn(move|| {
            let mut stream = stream.unwrap().try_clone().unwrap();
            stream.set_read_timeout(Some(Duration::new(30, 0))).unwrap();
            let mut buffer = vec![0; 4];
            match stream.read(&mut buffer) {
                Ok(_) => {}
                Err(_) => {
                    return;
                }
            }
            let response: String;
            // let the match nesting begin!
            match str::from_utf8(&buffer) {
                Err(_) => {
                    response = format!("Err: Bro I can't even read that");
                }
                Ok(input) => {
                    let received = input.lines().next().unwrap();
                    match received.parse::<usize>() {
                        Ok(n) => {
                            let (t_tx, t_rx) = mpsc::sync_channel(10);
                            match inner_tx.send(ThreadPhone { tx: t_tx, n: n }) {
                                Ok(_) => {}
                                Err(e) => {println!("{:?}", e);}
                            }
                            match t_rx.recv().unwrap() {
                                Ok(n) => {
                                    response = format!("Ok: {}", n);
                                }
                                Err(e) => {
                                    response = format!("Err: {}", e);
                                }
                            }
                        }
                        Err(_) => {
                            response = format!("'{}' is not an integer followed by a newline.", received);
                        }
                    }
                    println!("wanted {}, replied {:?}", received, response);
                }
            }
            // lazy slapping of newline on all responses sent back over the wire
            let response = format!("{}\n", response);
            stream.write(&response.as_bytes()).unwrap();
        });
    })
}
