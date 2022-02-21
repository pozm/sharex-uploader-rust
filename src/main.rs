use std::fs;
use std::future::Future;
use cuid::cuid;
use rocket::{catchers, Request, routes, get, catch, FromForm, post, request, Data, State, Config};
use rocket::data::ToByteUnit;
use rocket::form::{Form, Strict};
use rocket::fs::{NamedFile, TempFile};
use rocket::futures::future::IntoStream;
use rocket::futures::FutureExt;
use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::tokio::fs::File;
use rocket::response::stream::ReaderStream;

#[get("/")]
fn index() -> &'static str {
    "Hello, world!"
}

struct AuthKey(String);

struct UseAuthKey;
#[rocket::async_trait]
impl<'r> FromRequest<'r> for UseAuthKey {
    type Error = &'r str;

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        let key = request.guard::<&State<AuthKey>>().await.unwrap().inner();
        if key.0 == "" {
            request::Outcome::Success(UseAuthKey {})
        } else {
            let head_key = request.headers().get_one("auth_key").unwrap_or("");
            println!("provided key: {:?} vs actual: {:?}",head_key,key.0);
            if key.0 == head_key {
                request::Outcome::Success(UseAuthKey {})
            } else {
                request::Outcome::Failure((Status::Unauthorized,"invalid"))
            }
        }
    }
}


struct ReqHost(pub String);
#[rocket::async_trait]
impl<'r> FromRequest<'r> for &'r ReqHost {
    type Error = ();

    async fn from_request(request: &'r Request<'_>) -> request::Outcome<Self, Self::Error> {
        request::Outcome::Success(request.local_cache(|| {
            ReqHost(String::from(request.headers().get_one("host").unwrap()))
        }))
    }
}
#[get("/images/<id>")]
async fn get_image(id:&str) -> std::io::Result<NamedFile> {
    NamedFile::open(format!("./images/{}.png",&id)).await
}
#[post("/upload",data="<bin>")]
async fn upload_img(mut bin:Data<'_>, host_name:&ReqHost,_b:UseAuthKey) -> String {
    let id = cuid().unwrap();
    let path = std::env::current_dir().unwrap().join(format!("./images/{}.png",&id));
    let fi = File::create(path).await.unwrap();
    bin.open(32.megabytes()).stream_to(fi).await;
    format!("{}/images/{}",host_name.0,id)
}
#[catch(404)]
fn invalid_request(req: &Request) -> String {
    println!("{:?}",req.client_ip().unwrap());
    format!("hi, {:?} :)",req.client_ip().unwrap())
}

#[rocket::main]
async fn main() {
    println!("working file: {}",std::env::current_exe().unwrap().to_str().unwrap());
    let rocket = rocket::build();
    let auth : String = rocket.figment().extract_inner("auth_key").unwrap_or("".into());
    println!("auth: {:?}",auth);
    rocket.mount("/", routes![index,upload_img,get_image])
        .register("/", catchers![invalid_request])
        .manage(AuthKey(auth))
        .launch()
        .await
        .unwrap();
}
