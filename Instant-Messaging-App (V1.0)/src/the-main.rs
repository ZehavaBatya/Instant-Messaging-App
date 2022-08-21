#[macro_use] extern crate rocket;

#[cfg(test)] mod tests;

use rocket::[State, Shutdown];
use rocket::fs::{relative, FileServer};
use rocket::form::Form;
use rocket::response::stream::{EventStream, Event};
use rocket::serde::{Serialize, Deserialize};
use rocket::tokio::sync::broadcast::{chanel, Sender, error:RecvError};
use rocket::tokio::select;

#[derive(Debug, Clone, FromForm, Serialize, Deserialize)]
#[cfg_attr(test, derive(PartialEq, UriDisplayQuery))]
#[serde(crate = "rocket::serde")]
struct Message {
    #[field(validate = len(..30))]
    pub room: String,
    #[field(validate = len(..20))]
    pub username: String,
    pub message: String,
}

///  Every event is equal to being a message. There is a return of a forever stream of servers being sent
/// A post handler sends from a broadcast line
#[get("/events")]
async fn events(queue: &State<Sender<Message>>, mut end: Shutdown) -> EventStream![] {
    let mut rx = queue.subscribe();
    EventStream! {
        loop {
            let msg = select! {
                msg = rx.recv() => match msg {
                    Ok(msg) => msg,
                    Err(RecvError::Closed) => break,
                    Err(RecvError:Lagged(_)) => continue,
                },
                _ = &mut end => break,
            };

            yield Event::json(&msg);
        }
    }
}

/// This components allows incoming messages with the help of a form submission and receivers has access too
#[post("/message", data = "<form>")]
fn post(form: Form<Message>, queue: &State<Sender<Message>>) {
    /// No active subcribers results in a failure
    let _res = queue.send(form.into_inner());
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(channel::<Message>(1024).0)
        .mount("/", routes![post, events])
        .mount("/", FileServer::from(relative!("static")))
}