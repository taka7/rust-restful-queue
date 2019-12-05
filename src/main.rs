use crossbeam_queue;
use std::sync::atomic::Ordering::Relaxed;
use std::sync::atomic::{AtomicBool, AtomicI32};
use std::sync::Arc;

#[macro_use]
extern crate tower_web;

#[macro_use]
extern crate serde_json;

#[derive(Debug, PartialEq)]
enum Request {
    AA(f32),
    BB(f32),
}

struct JsonResource {
    queue: Arc<crossbeam_queue::ArrayQueue<Request>>,
    a: Arc<AtomicI32>,
    b: Arc<AtomicBool>,
}

#[derive(Debug, Extract)]
struct Req {
    value: f32,
}

#[derive(Debug, Response)]
struct Status {
    a: i32,
    b: bool,
}

impl_web! {
    impl JsonResource {
        #[put("/aa")]
        #[content_type("application/json")]
        fn aa(&self, body: Req) -> Result<serde_json::Value, ()> {
            let _ = self.queue.push(Request::AA(body.value));

            Ok(json!({
                "message": "Ok",
            }))
        }

        #[put("/bb")]
        #[content_type("application/json")]
        fn bb(&self, body: Req) -> Result<serde_json::Value, ()> {
            let _ = self.queue.push(Request::BB(body.value));

            Ok(json!({
                "message": "Ok",
            }))

        }

        #[get("/status")]
        #[content_type("application/json")]
        fn status(&self) -> Result<Status, ()> {
            Ok(Status { a: self.a.load(Relaxed), b: self.b.load(Relaxed) })
        }
    }
}

impl JsonResource {
    fn new(
        queue: Arc<crossbeam_queue::ArrayQueue<Request>>,
        a: Arc<AtomicI32>,
        b: Arc<AtomicBool>,
    ) -> Self {
        JsonResource { queue, a, b }
    }
}

pub fn main() {
    let req_q = Arc::new(crossbeam_queue::ArrayQueue::<Request>::new(10));
    let status_q = Arc::new(crossbeam_queue::ArrayQueue::<Status>::new(10));
    let st_a = Arc::new(AtomicI32::new(0));
    let st_b = Arc::new(AtomicBool::new(false));

    let req_q_http = req_q.clone();
    let st_a_http = st_a.clone();
    let st_b_http = st_b.clone();
    let _http_server_handler = std::thread::spawn(move || {
        let addr = "127.0.0.1:8080".parse().expect("Invalid address");
        println!("Listening on http://{}", addr);

        tower_web::ServiceBuilder::new()
            .resource(JsonResource::new(req_q_http, st_a_http, st_b_http))
            .run(&addr)
            .unwrap();
    });

    let status_q_2 = status_q.clone();
    let _status_queue_handler = std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if let Ok(ref v) = status_q_2.pop() {
            st_a.store(v.a, Relaxed);
            st_b.store(v.b, Relaxed);
        }
    });

    let mut counter = 0;
    loop {
        std::thread::sleep(std::time::Duration::from_secs(1));
        if let Ok(ref v) = req_q.pop() {
            println!("Got request: {:?}", v);
        }
        let _ = status_q.push(Status {
            a: counter,
            b: counter % 2 == 0,
        });
        counter += 1;
    }
}
