#[macro_use]
extern crate nickel;

use std::io::prelude::*;
use std::net::TcpListener;
use std::str;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use nickel::status::StatusCode;
use nickel::{HttpRouter, JsonBody, Nickel};
use serde::{Deserialize, Serialize};
use serde_json::Value as AnyJson;

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

#[derive(Deserialize)]
struct FibNRequest {
    n: usize,
}

#[derive(Serialize)]
struct FibOkResponse {
    ok: u64,
}

#[derive(Serialize)]
struct FibErrResponse {
    err: String,
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
                    let (tx, rx) = mpsc::sync_channel(1);
                    inner_tx.send(ThreadPhone { tx, n }).unwrap();
                    // the actual fibber answer, formatted to match the "interface"
                    match rx.recv().unwrap() {
                        Ok(n) => Ok(n.to_string()),
                        Err(e) => Err(e),
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
            Err(String::from("Bro I can't even read that"))
        }
    }
}

// eventually should replace the `.read` with a `.read_to_string`, since that's where we're
// eventually trying to get anyway, but for now the unbounded `.read` works fine.
#[allow(clippy::unused_io_amount)]
fn main() {
    let mut nickel = Nickel::new();
    let (tx, rx) = mpsc::sync_channel(10);
    let nickel_tx = tx.clone();
    let sock_addr = "0.0.0.0:1234";
    // panic will ensue if the port is in use :D
    let listener = TcpListener::bind(sock_addr).unwrap();
    println!("Socket server listening at {}", sock_addr);

    // hand over the cache and receiver to a single worker thread
    // connections are concurrent, but generating and caching fib values isn't
    thread::spawn(move || {
        let mut cache = async_fibber::fib_cache();
        for tp in rx.iter() {
            let tp = tp as ThreadPhone<_>;
            tp.tx.send(async_fibber::fib(tp.n, &mut cache)).unwrap();
        }
    });

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
    thread::spawn(move || {
        let mut incoming = listener.incoming();
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
                stream.write_all(&formatted_response.as_bytes()).unwrap();
                // log what happened
                println!("sent {}", formatted_response.trim());
            });
        }
    });

    // So all that socket stuff up above was some v1 business. Given that
    // I actually am running all of this in an openshift, it seems reasonable
    // to expose the service in an OpenShift-friendly, RESTy rusty sort of way.
    // So we kick all of the socket stuff into threads, and meanwhile...REST
    // API with nickel I guess? On with the crowbar...ing!
    nickel.get(
        "/",
        middleware!(String::from(
            "POST some JSON to do fibby stuff, e.g. {{\"n\": <some_int>}}"
        )),
    );
    nickel.post(
        "/",
        middleware! {|request, response|
            match request.json_as::<FibNRequest>() {
                Ok(fib_request) => {
                    // yeah, we're still going to send all the work back to the threaded worker though
                    let (req_tx, req_rx) = mpsc::sync_channel(1);
                    nickel_tx.send(ThreadPhone { tx: req_tx, n: fib_request.n }).unwrap();
                    match req_rx.recv().unwrap() {
                        Ok(ok) => {
                            let result = FibOkResponse { ok };
                            (StatusCode::Ok, serde_json::to_string_pretty(&result).unwrap())
                        }
                        Err(err) => {
                            let result = FibErrResponse { err };
                            (StatusCode::Ok, serde_json::to_string_pretty(&result).unwrap())
                        }
                    }
                }
                Err(_) => {
                    match request.json_as::<AnyJson>() {
                        Ok(_) => {
                            let result = FibErrResponse { err: "Expects JSON in form '{\"n\": <fibonacci index int to look up>}'".to_string() };
                            (StatusCode::Ok, serde_json::to_string_pretty(&result).unwrap())
                        }
                        Err(_) => (StatusCode::BadRequest, String::from("Bro that isn't even JSON"))
                    }
                }
            }
        },
    );

    // time to hardcode another port!
    nickel.listen("0.0.0.0:8080").unwrap();
}
