use std::fs;
use std::future::Future;
use std::path::Path;
use cuid::cuid;
use rocket::{catchers, Request, routes, get, catch, FromForm, post, request, Data, State, Config,delete};
use rocket::data::ToByteUnit;
use rocket::form::{Form, Strict};
use rocket::fs::{FileServer, NamedFile, TempFile,relative};
use rocket::futures::future::IntoStream;
use rocket::futures::FutureExt;
use rocket::http::Status;
use rocket::request::FromRequest;
use rocket::tokio::fs::{File, read_dir, remove_file};
use rocket::response::stream::ReaderStream;
use rocket::tokio::task::spawn_blocking;
use rocket::serde::{Serialize, json::Json};

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
#[derive(Serialize)]
struct ImagesObj(Vec<String>);

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
async fn get_image(id:&str) -> NamedFile {
    NamedFile::open(format!("./images/{}.png",&id)).await.unwrap()
}
#[delete("/images/<id>")]
async fn delete_image(id:&str,_b:UseAuthKey) -> std::io::Result<()> {
    remove_file(format!("./images/{}.png",&id)).await
}
#[get("/images")]
async fn get_images() -> Json<ImagesObj> {
    let images = spawn_blocking(|| {
        let imgs = std::fs::read_dir("./images").unwrap();
        let mut vec:Vec<String> = vec!{};
        for img in imgs {
            vec.push(img.unwrap().path().file_stem().unwrap().to_str().unwrap().to_string())
        }
        vec
    }).await.unwrap();
    println!("{:#?}",images);
    Json(ImagesObj(images))
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
    if cfg!(not(debug_assertions))
    {
        rocket.mount("/api/", routes![upload_img,get_image,get_images,delete_image])
            .register("/", catchers![invalid_request])
            .manage(AuthKey(auth))
            .mount("/", FileServer::from(relative!("client/dist")))
    } else {
        rocket.mount("/api/", routes![upload_img,get_image,get_images,delete_image])
            .register("/", catchers![invalid_request])
            .manage(AuthKey(auth))
    }
        .launch()
        .await
        .unwrap();
}
