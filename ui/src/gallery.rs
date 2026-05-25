use dioxus::prelude::*;
use serde::Deserialize;

const CSS: Asset = asset!("/assets/styling/gallery.css");

#[derive(Deserialize, Debug, Clone)]
struct Post {
    id: u64,
    preview_url: String,
    file_url: String,
}

#[component]
pub fn Gallery() -> Element {
    let images = use_resource(|| async {
        let response = reqwest::get("https://yande.re/post.json")
            .await
            .ok()?
            .json::<Vec<Post>>()
            .await
            .ok();
        response
    });
    rsx! {
    document::Link { rel: "stylesheet", href: CSS }
    div {
        id: "gallery",
        match images.read().as_ref() {
            Some(Some(posts)) => posts.iter().map(|post| rsx! {
                div { class: "image-card",
                    img { src: "{post.preview_url}", alt: "preview" }
                    a {
                        href: "{post.file_url}",
                        download: true,
                        "Download Original"
                    }
                }
            }).collect::<Vec<_>>().into_iter(),
            _ => std::iter::once(rsx! { p { "Loading..." } })
        }
    }
}
