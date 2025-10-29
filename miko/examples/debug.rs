use miko::macros::*;
use miko::*;
use serde_json::json;

#[get("/async")]
async fn async_handler(#[query] recursive: Option<bool>) {
    let recursive = recursive.unwrap_or(false);
    // list files in the current directory

    json!({
        "message": "This is an async handler",
        "files": std::fs::read_dir(".")
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<String>>(),
    })
}

#[miko]
async fn main() {
    router.cors_any();
}
