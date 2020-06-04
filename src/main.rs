use std::io::prelude::*;
use std::net::TcpListener;
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

mod async_fibber;

// To enable bidirectional communications from withing threads, so
// that all threads benefit from the fib cache, threads call into
// the parent over an mpsc channel with a ThreadPhone. This has its
// own mpsc channel embedded in it, as well as the requested fib index
// to look up. The answer is calculated, and then sent back over the
// ThreadPhone.tx channel. This admittedly results in cache lookups
// being a potential bottleneck, but it's going to be a pretty wide
// neck. I may (but probably won't) profile it at some point to see.
struct ThreadPhone<T> {
    tx: mpsc::SyncSender<T>,
    n: usize,
}

// breaking this out into its own function was fun, if only to appreciate
// what the actual type of the nested SyncSender is after resolving the
// generic types. All this does is take the input buffer from the stream
// and convert it to a Result.
fn process_buffer(
    inner_tx: mpsc::SyncSender<ThreadPhone<Result<u64, String>>>,
    buffer: Vec<u8>,
) -> Result<String, String> {
    // let the match nesting begin!
    match str::from_utf8(&buffer) {
        Ok(input) => {
            let received = input.lines().next().unwrap();
            match received.parse::<usize>() {
                Ok(n) => {
                    let (t_tx, t_rx) = mpsc::sync_channel(10);
                    inner_tx.send(ThreadPhone { tx: t_tx, n: n }).unwrap();
                    // the actual fibber answer, formatted to match the "interface"
                    match t_rx.recv().unwrap() {
                        Ok(n) => Ok(format!("{}", n)),
                        Err(e) => Err(format!("{}", e)),
                    }
                }
                Err(_) => Err(format!(
                    "'{}' is not an integer followed by a newline",
                    received
                )),
            }
        }
        Err(_) => {
            // Bro, I couldn't even pretend to make a utf8 string out of you sent
            Err(format!("Bro I can't even read that"))
        }
    }
}

fn main() {
    let (tx, rx) = mpsc::sync_channel(10);
    let mut cache = async_fibber::fib_cache();
    let addr = "0.0.0.0:1234";
    // panic will ensue if the port is in use :D
    let listener = TcpListener::bind(addr).unwrap();
    println!("Listening at {}", addr);

    // hand over the cache and receiver to a single worker thread
    // connections are concurrent, but generating and caching fib values isn't
    thread::spawn(move || {
        for tp in rx.iter() {
            let tp = tp as ThreadPhone<_>;
            tp.tx.send(async_fibber::fib(tp.n, &mut cache)).unwrap();
        }
    });

    let mut incoming = listener.incoming();
    // So I'll be honest here. I'm new to this whole "if/while let" thing. I'm pretty sure
    // that what I'm saying is that "while junk coming off of this iterator is not a "None"
    // option, and is not a "Some" option wrapping an "Err" result, expose "stream" in
    // the looping scope as the unwrapped value. I'm not entirely sure what happens to
    // all those "None" values and "Some(Err(e))" values, though. I *assume* they got
    // dropped on the floor, and that I won't end up with some kind of stuck iterator.
    // I expect to look back on this comment in a few years (or days? I dunno) and laugh,
    // as this is certainly contained somewhere in the firehose of docs I consumed over
    // the past few days. Could be that when the while let hits an Err then main() panics,
    // which would be bad. I should probably test this in isolation, but probably won't.
    while let Some(Ok(mut stream)) = incoming.next() {
        // pass off incoming connections to a thread asap, at which point the mpsc
        // channel + ThreadPhone is used to pass work up to the worker thread.
        // Thread panics end up getting printed, which is fine for this example,
        // so everything that doesn't end up generating an Ok/Err response to the
        // socket is just .unwrap'd.
        let inner_tx = tx.clone();
        thread::spawn(move || {
            let mut buffer = vec![0; 16];
            let formatted_response: String;
            stream.set_read_timeout(Some(Duration::new(30, 0))).unwrap();
            stream.read(&mut buffer).unwrap();
            match process_buffer(inner_tx, buffer) {
                Ok(msg) => {
                    formatted_response = format!("Ok: {}\n", msg);
                }
                Err(msg) => {
                    formatted_response = format!("Err: {}\n", msg);
                }
            }
            stream.write(&formatted_response.as_bytes()).unwrap();
            // log what happened
            println!("sent {}", formatted_response.trim());
        });
    }
}
